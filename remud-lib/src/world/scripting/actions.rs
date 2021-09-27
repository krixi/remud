use crate::{
    engine::persist::{self, Updates},
    web::scripts::ScriptError,
    world::scripting::{
        CompilationError, CompiledScript, FailedScript, Script, ScriptAst, ScriptEngine,
        ScriptName, Scripts,
    },
};

use bevy_ecs::prelude::*;
use either::Either;
use rhai::ParseError;

pub async fn create_script(
    world: &mut World,
    script: Script,
) -> Result<Option<ParseError>, ScriptError> {
    tracing::debug!("Creating {:?}.", script.name);

    if world
        .get_resource::<Scripts>()
        .unwrap()
        .by_name(&script.name)
        .is_some()
    {
        return Err(ScriptError::DuplicateName);
    }

    let engine = world.get_resource::<ScriptEngine>().unwrap().get();
    let (id, error) = match engine.read().await.compile(script.code.as_str()) {
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
) -> Result<(Script, Option<ParseError>), ScriptError> {
    tracing::debug!("Retrieving {:?}.", name);

    let script_entity =
        if let Some(entity) = world.get_resource::<Scripts>().unwrap().by_name(&name) {
            entity
        } else {
            return Err(ScriptError::ScriptNotFound);
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

pub async fn update_script(
    world: &mut World,
    script: Script,
) -> Result<Option<ParseError>, ScriptError> {
    tracing::debug!("Updating {:?}.", script.name);

    let script_entity = if let Some(entity) = world
        .get_resource::<Scripts>()
        .unwrap()
        .by_name(&script.name)
    {
        entity
    } else {
        return Err(ScriptError::ScriptNotFound);
    };

    let engine = world.get_resource::<ScriptEngine>().unwrap().get();
    let error = match engine.read().await.compile(script.code.as_str()) {
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

pub fn delete_script(world: &mut World, name: ScriptName) -> Result<(), ScriptError> {
    let script_entity =
        if let Some(entity) = world.get_resource::<Scripts>().unwrap().by_name(&name) {
            entity
        } else {
            return Err(ScriptError::ScriptNotFound);
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

pub async fn compile_scripts(world: &mut World) {
    let engine = world.get_resource::<ScriptEngine>().unwrap().get();
    let engine = engine.read().await;
    let mut results = Vec::new();
    for (entity, script) in world
        .query_filtered::<(Entity, &Script), (Without<ScriptAst>, Without<CompilationError>)>()
        .iter(world)
    {
        match engine.compile(script.as_str()) {
            Ok(ast) => results.push((entity, Either::Left(ast))),
            Err(error) => results.push((entity, Either::Right(error))),
        }
    }

    for (entity, result) in results {
        match result {
            Either::Left(ast) => {
                world.entity_mut(entity).insert(ScriptAst::from(ast));
            }
            Either::Right(error) => {
                world
                    .entity_mut(entity)
                    .insert(CompilationError::from(error));
            }
        }
    }
}
