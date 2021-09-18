use std::sync::{Arc, RwLock};

use crate::{
    engine::persist::{self, Updates},
    world::{
        action::ActionEvent,
        scripting::{
            CompiledScript, FailedScript, Script, ScriptAst, ScriptEngine, ScriptError, ScriptName,
            Scripts,
        },
    },
};

use anyhow::bail;
use bevy_ecs::prelude::*;
use rhai::{Dynamic, ParseError, Scope};

pub fn create_script(world: &mut World, script: Script) -> anyhow::Result<Option<ParseError>> {
    tracing::info!("Creating {:?}.", script.name);

    if world
        .get_resource::<Scripts>()
        .unwrap()
        .by_name(&script.name)
        .is_some()
    {
        bail!("Script {:?} already exists.", script.name)
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
                    error: ScriptError {
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

pub fn read_script(world: &World, name: ScriptName) -> anyhow::Result<Script> {
    tracing::info!("Retrieving {:?}.", name);

    let script_entity =
        if let Some(entity) = world.get_resource::<Scripts>().unwrap().by_name(&name) {
            entity
        } else {
            bail!("Script {:?} not found.", name);
        };

    let script = world.get::<Script>(script_entity).unwrap().clone();

    Ok(script)
}

pub fn read_all_scripts(world: &mut World) -> anyhow::Result<Vec<Script>> {
    let mut scripts = Vec::new();

    for script in world.query::<&Script>().iter(world) {
        scripts.push(script.clone());
    }

    tracing::info!("Retrieved {} scripts.", scripts.len());

    Ok(scripts)
}

pub fn run_script(
    world: Arc<RwLock<World>>,
    event: &ActionEvent,
    entity: Entity,
    script: ScriptName,
) {
    if let Some(script) = world
        .read()
        .unwrap()
        .get_resource::<Scripts>()
        .unwrap()
        .by_name(&script)
    {
        if let Some(ast) = world
            .read()
            .unwrap()
            .get::<ScriptAst>(script)
            .map(|script_ast| script_ast.ast.clone())
        {
            let engine = world
                .read()
                .unwrap()
                .get_resource::<ScriptEngine>()
                .unwrap()
                .get();

            let mut scope = Scope::new();
            scope.push_constant("SELF", entity);
            scope.push_constant("WORLD", world.clone());
            scope.push_constant("EVENT", event.clone());

            match world.try_write() {
                Ok(_) => tracing::info!("can write"),
                Err(_) => tracing::info!("can't write"),
            }

            engine
                .read()
                .unwrap()
                .consume_ast_with_scope(&mut scope, &ast)
                .unwrap();
        } else {
            tracing::warn!(
                "Skipping execution of {:?}, compiled script not found.",
                script
            );
        };
    }
}

pub fn run_pre_script(
    world: Arc<RwLock<World>>,
    event: &ActionEvent,
    entity: Entity,
    script: ScriptName,
) -> bool {
    if let Some(script) = world
        .read()
        .unwrap()
        .get_resource::<Scripts>()
        .unwrap()
        .by_name(&script)
    {
        if let Some(ast) = world
            .read()
            .unwrap()
            .get::<ScriptAst>(script)
            .map(|script_ast| script_ast.ast.clone())
        {
            let engine = world
                .read()
                .unwrap()
                .get_resource::<ScriptEngine>()
                .unwrap()
                .get();

            let mut scope = Scope::new();
            scope.push_constant("SELF", entity);
            scope.push_constant("WORLD", world.clone());
            scope.push_constant("EVENT", event.clone());
            scope.push_dynamic("allow_action", Dynamic::from(true));

            engine
                .read()
                .unwrap()
                .consume_ast_with_scope(&mut scope, &ast)
                .unwrap();

            return scope.get_value("allow_action").unwrap();
        } else {
            tracing::warn!(
                "Skipping execution of {:?}, compiled script not found.",
                script
            );
        };
    }

    false
}

pub fn update_script(world: &mut World, script: Script) -> anyhow::Result<Option<ParseError>> {
    tracing::info!("Updating {:?}.", script.name);

    let script_entity = if let Some(entity) = world
        .get_resource::<Scripts>()
        .unwrap()
        .by_name(&script.name)
    {
        entity
    } else {
        bail!("Script {:?} does not exists.", script.name)
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
                .remove::<ScriptError>();
            None
        }
        Err(error) => {
            world
                .entity_mut(script_entity)
                .insert_bundle(FailedScript {
                    script: script.clone(),
                    error: ScriptError {
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

pub fn delete_script(world: &mut World, name: ScriptName) -> anyhow::Result<()> {
    let script_entity =
        if let Some(entity) = world.get_resource::<Scripts>().unwrap().by_name(&name) {
            entity
        } else {
            bail!("Script {:?} does not exists.", name)
        };

    tracing::info!("Deleting script for web: {:?}", name);

    world.despawn(script_entity);

    world.get_resource_mut::<Scripts>().unwrap().remove(&name);

    world
        .get_resource_mut::<Updates>()
        .unwrap()
        .queue(persist::script::Delete::new(name.to_string()));

    Ok(())
}
