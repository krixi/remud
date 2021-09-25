use std::sync::{Arc, RwLock};

use crate::world::{
    action::Action,
    ecs::SharedWorld,
    scripting::{modules::Me, ExecutionErrors, ScriptAst, ScriptEngine, ScriptName, Scripts},
};

use bevy_ecs::prelude::*;
use rhai::{Dynamic, Engine, Scope, AST};

pub fn run_init_script(world: SharedWorld, entity: Entity, script: ScriptName) {
    let (ast, engine) = match prepare_script_execution(&*world.read().unwrap(), &script) {
        Some(results) => results,
        None => return,
    };

    let mut scope = Scope::new();
    scope.push_constant(
        "SELF",
        Me {
            world: world.clone(),
            entity,
        },
    );
    scope.push_constant("WORLD", world.clone());

    match engine
        .read()
        .unwrap()
        .consume_ast_with_scope(&mut scope, &ast)
    {
        Ok(_) => (),
        Err(error) => {
            tracing::warn!("Init script {} execution error: {}", script, error);

            if let Some(mut errors) = world.write().unwrap().get_mut::<ExecutionErrors>(entity) {
                errors.insert(script, error)
            } else {
                world
                    .write()
                    .unwrap()
                    .get_entity_mut(entity)
                    .unwrap()
                    .insert(ExecutionErrors::new_with_error(script, error));
            }
        }
    };
}

pub fn run_post_event_script(
    world: SharedWorld,
    event: &Action,
    entity: Entity,
    script: ScriptName,
) {
    let (ast, engine) = match prepare_script_execution(&*world.read().unwrap(), &script) {
        Some(results) => results,
        None => return,
    };

    let mut scope = Scope::new();
    scope.push_constant(
        "SELF",
        Me {
            world: world.clone(),
            entity,
        },
    );
    scope.push_constant("WORLD", world.clone());
    scope.push_constant("EVENT", event.clone());

    match engine
        .read()
        .unwrap()
        .consume_ast_with_scope(&mut scope, &ast)
    {
        Ok(_) => (),
        Err(error) => {
            tracing::warn!("Post-event script {} execution error: {}", script, error);

            let mut world = world.write().unwrap();

            if let Some(mut errors) = world.get_mut::<ExecutionErrors>(entity) {
                errors.insert(script, error)
            } else {
                world
                    .get_entity_mut(entity)
                    .unwrap()
                    .insert(ExecutionErrors::new_with_error(script, error));
            }
        }
    };
}

pub fn run_pre_event_script(
    world: SharedWorld,
    event: &Action,
    entity: Entity,
    script: ScriptName,
) -> bool {
    let (ast, engine) = match prepare_script_execution(&*world.read().unwrap(), &script) {
        Some(results) => results,
        None => return true,
    };

    let mut scope = Scope::new();
    scope.push_constant(
        "SELF",
        Me {
            world: world.clone(),
            entity,
        },
    );
    scope.push_constant("WORLD", world.clone());
    scope.push_constant("EVENT", event.clone());
    scope.push_dynamic("allow_action", Dynamic::from(true));

    match engine
        .read()
        .unwrap()
        .consume_ast_with_scope(&mut scope, &ast)
    {
        Ok(_) => (),
        Err(error) => {
            tracing::warn!("Pre-event script {} execution error: {}", script, error);

            if let Some(mut errors) = world.write().unwrap().get_mut::<ExecutionErrors>(entity) {
                errors.insert(script, error)
            } else {
                world
                    .write()
                    .unwrap()
                    .get_entity_mut(entity)
                    .unwrap()
                    .insert(ExecutionErrors::new_with_error(script, error));
            }
        }
    }

    scope.get_value("allow_action").unwrap()
}

pub fn run_timed_script(world: SharedWorld, entity: Entity, script: ScriptName) {
    let (ast, engine) = match prepare_script_execution(&*world.read().unwrap(), &script) {
        Some(results) => results,
        None => return,
    };

    let mut scope = Scope::new();
    scope.push_constant(
        "SELF",
        Me {
            world: world.clone(),
            entity,
        },
    );
    scope.push_constant("WORLD", world.clone());

    match engine
        .read()
        .unwrap()
        .consume_ast_with_scope(&mut scope, &ast)
    {
        Ok(_) => (),
        Err(error) => {
            tracing::warn!("Timed script {} execution error: {}", script, error);
            if let Some(mut errors) = world.write().unwrap().get_mut::<ExecutionErrors>(entity) {
                errors.insert(script, error)
            } else {
                world
                    .write()
                    .unwrap()
                    .get_entity_mut(entity)
                    .unwrap()
                    .insert(ExecutionErrors::new_with_error(script, error));
            }
        }
    };
}

fn prepare_script_execution(
    world: &World,
    script: &ScriptName,
) -> Option<(AST, Arc<RwLock<Engine>>)> {
    let script = {
        if let Some(script) = world.get_resource::<Scripts>().unwrap().by_name(script) {
            script
        } else {
            tracing::warn!(
                "Skipping execution of {:?}, unable to find named script.",
                script
            );
            return None;
        }
    };

    let ast = {
        if let Some(ast) = world
            .get::<ScriptAst>(script)
            .map(|script_ast| script_ast.ast.clone())
        {
            ast
        } else {
            tracing::warn!(
                "Skipping execution of {:?}, compiled script not found.",
                script
            );
            return None;
        }
    };

    let engine = world.get_resource::<ScriptEngine>().unwrap().get();

    Some((ast, engine))
}
