#![allow(dead_code)]
/// test scripts prototype interface
/// init scripts are only run when an object is created, or on load, or when the init command is used, they are never run on prototypes
fn test_script_prototype_attach_init() {}
fn test_script_prototype_attach_pre() {}
fn test_script_prototype_attach_pre_disallow_action() {}
fn test_script_prototype_attach_post() {}
fn test_script_prototype_attach_timer() {}
fn test_script_prototype_detach() {}
