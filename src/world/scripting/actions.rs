use std::sync::{Arc, RwLock};

use crate::{
    engine::persist::{self, Updates},
    web,
    world::{
        action::Action,
        scripting::{
            CompilationError, CompiledScript, FailedScript, Script, ScriptAst, ScriptEngine,
            ScriptName, Scripts,
        },
    },
};

use bevy_ecs::prelude::*;
use rhai::{Dynamic, ParseError, Scope};

pub fn create_script(world: &mut World, script: Script) -> Result<Option<ParseError>, web::Error> {
    tracing::debug!("Creating {:?}.", script.name);

    if world
        .get_resource::<Scripts>()
        .unwrap()
        .by_name(&script.name)
        .is_some()
    {
        return Err(web::Error::DuplicateName);
    }

    let engine = world.get_resource::<ScriptEngine>().unwrap().get();
    let (id, error) = match engine.read().unwrap().compile(script.code.as_str()) {
        Ok(ast) => {
            let id = world
                .spawn()
                .insert_bundle(CompiledScript {
                    script: script.clone(),
                    ast: ScriptAst { ast },
                })
                .id();
            (id, None)
        }
        Err(error) => {
            let id = world
                .spawn()
                .insert_bundle(FailedScript {
                    script: script.clone(),
                    error: CompilationError {
                        error: error.clone(),
                    },
                })
                .id();
            (id, Some(error))
        }
    };

    world
        .get_resource_mut::<Scripts>()
        .unwrap()
        .insert(script.name.clone(), id);

    world
        .get_resource_mut::<Updates>()
        .unwrap()
        .queue(persist::script::Create::new(
            script.name.to_string(),
            script.trigger.to_string(),
            script.code,
        ));

    Ok(error)
}

pub fn read_script(world: &World, name: ScriptName) -> Result<Script, web::Error> {
    tracing::debug!("Retrieving {:?}.", name);

    let script_entity =
        if let Some(entity) = world.get_resource::<Scripts>().unwrap().by_name(&name) {
            entity
        } else {
            return Err(web::Error::ScriptNotFound);
        };

    let script = world.get::<Script>(script_entity).unwrap().clone();

    Ok(script)
}

pub fn read_all_scripts(world: &mut World) -> Vec<Script> {
    let mut scripts = Vec::new();

    for script in world.query::<&Script>().iter(world) {
        scripts.push(script.clone());
    }

    tracing::debug!("Retrieved {} scripts.", scripts.len());

    scripts
}

pub fn run_script(world: Arc<RwLock<World>>, event: &Action, entity: Entity, script: ScriptName) {
    let script = {
        if let Some(script) = world
            .read()
            .unwrap()
            .get_resource::<Scripts>()
            .unwrap()
            .by_name(&script)
        {
            script
        } else {
            tracing::warn!(
                "Skipping execution of {:?}, unable to find named script.",
                script
            );
            return;
        }
    };

    let ast = {
        if let Some(ast) = world
            .read()
            .unwrap()
            .get::<ScriptAst>(script)
            .map(|script_ast| script_ast.ast.clone())
        {
            ast
        } else {
            tracing::warn!(
                "Skipping execution of {:?}, compiled script not found.",
                script
            );
            return;
        }
    };

    let engine = {
        let engine = world
            .read()
            .unwrap()
            .get_resource::<ScriptEngine>()
            .unwrap()
            .get();

        engine
    };

    let mut scope = Scope::new();
    scope.push_constant("SELF", entity);
    scope.push_constant("WORLD", world);
    scope.push_constant("EVENT", event.clone());

    match engine
        .read()
        .unwrap()
        .consume_ast_with_scope(&mut scope, &ast)
    {
        Ok(_) => (),
        Err(e) => tracing::warn!("Script execution error: {}", e),
    };
}

pub fn run_pre_script(
    world: Arc<RwLock<World>>,
    event: &Action,
    entity: Entity,
    script: ScriptName,
) -> bool {
    let script = {
        if let Some(script) = world
            .read()
            .unwrap()
            .get_resource::<Scripts>()
            .unwrap()
            .by_name(&script)
        {
            script
        } else {
            tracing::warn!(
                "Skipping execution of {:?}, unable to find named script.",
                script
            );
            return true;
        }
    };

    let ast = {
        if let Some(ast) = world
            .read()
            .unwrap()
            .get::<ScriptAst>(script)
            .map(|script_ast| script_ast.ast.clone())
        {
            ast
        } else {
            tracing::warn!(
                "Skipping execution of {:?}, compiled script not found.",
                script
            );
            return true;
        }
    };

    let engine = {
        let engine = world
            .read()
            .unwrap()
            .get_resource::<ScriptEngine>()
            .unwrap()
            .get();

        engine
    };

    let mut scope = Scope::new();
    scope.push_constant("SELF", entity);
    scope.push_constant("WORLD", world);
    scope.push_constant("EVENT", event.clone());
    scope.push_dynamic("allow_action", Dynamic::from(true));

    engine
        .read()
        .unwrap()
        .consume_ast_with_scope(&mut scope, &ast)
        .unwrap();

    scope.get_value("allow_action").unwrap()
}

pub fn update_script(world: &mut World, script: Script) -> Result<Option<ParseError>, web::Error> {
    tracing::debug!("Updating {:?}.", script.name);

    let script_entity = if let Some(entity) = world
        .get_resource::<Scripts>()
        .unwrap()
        .by_name(&script.name)
    {
        entity
    } else {
        return Err(web::Error::ScriptNotFound);
    };

    let engine = world.get_resource::<ScriptEngine>().unwrap().get();
    let error = match engine.read().unwrap().compile(script.code.as_str()) {
        Ok(ast) => {
            world
                .entity_mut(script_entity)
                .insert_bundle(CompiledScript {
                    script: script.clone(),
                    ast: ScriptAst { ast },
                })
                .remove::<CompilationError>();
            None
        }
        Err(error) => {
            world
                .entity_mut(script_entity)
                .insert_bundle(FailedScript {
                    script: script.clone(),
                    error: CompilationError {
                        error: error.clone(),
                    },
                })
                .remove::<ScriptAst>();
            Some(error)
        }
    };

    world
        .get_resource_mut::<Updates>()
        .unwrap()
        .queue(persist::script::Update::new(
            script.name.to_string(),
            script.trigger.to_string(),
            script.code,
        ));

    Ok(error)
}

pub fn delete_script(world: &mut World, name: ScriptName) -> Result<(), web::Error> {
    let script_entity =
        if let Some(entity) = world.get_resource::<Scripts>().unwrap().by_name(&name) {
            entity
        } else {
            return Err(web::Error::ScriptNotFound);
        };

    tracing::debug!("Deleting: {:?}", name);

    world.despawn(script_entity);

    world.get_resource_mut::<Scripts>().unwrap().remove(&name);

    world
        .get_resource_mut::<Updates>()
        .unwrap()
        .queue(persist::script::Delete::new(name.to_string()));

    Ok(())
}
