use crate::{
    engine::persist::{self, Updates},
    web,
    world::scripting::{
        CompilationError, CompiledScript, FailedScript, Script, ScriptAst, ScriptEngine,
        ScriptName, Scripts,
    },
};

use bevy_ecs::prelude::*;
use rhai::ParseError;

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
        .persist(persist::script::Create::new(
            script.name.to_string(),
            script.trigger.to_string(),
            script.code,
        ));

    Ok(error)
}

pub fn read_script(
    world: &World,
    name: ScriptName,
) -> Result<(Script, Option<ParseError>), web::Error> {
    tracing::debug!("Retrieving {:?}.", name);

    let script_entity =
        if let Some(entity) = world.get_resource::<Scripts>().unwrap().by_name(&name) {
            entity
        } else {
            return Err(web::Error::ScriptNotFound);
        };

    let script = world.get::<Script>(script_entity).unwrap().clone();
    let err = world
        .get::<CompilationError>(script_entity)
        .map(|e| e.error.clone());

    Ok((script, err))
}

pub fn read_all_scripts(world: &mut World) -> Vec<(Script, Option<ParseError>)> {
    let mut scripts = Vec::new();

    for (script, error) in world
        .query::<(&Script, Option<&CompilationError>)>()
        .iter(world)
    {
        scripts.push((script.clone(), error.map(|c| c.error.clone())));
    }

    tracing::debug!("Retrieved {} scripts.", scripts.len());

    scripts
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
        .persist(persist::script::Update::new(
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
        .persist(persist::script::Remove::new(name.to_string()));

    Ok(())
}
