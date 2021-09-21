use std::{collections::HashMap, fmt};

use bevy_ecs::prelude::Entity;
use itertools::Itertools;
use lazy_static::lazy_static;
use regex::{Regex, Replacer};

use crate::{
    text::{word_list, Tokenizer},
    world::{
        action::{
            attributes::parse_stats,
            communicate::{parse_me, parse_say, parse_send},
            immortal::{
                object::parse_object, player::parse_player, room::parse_room, script::parse_script,
            },
            movement::{parse_teleport, Move},
            object::{parse_drop, parse_get, Inventory},
            observe::{parse_look, Exits, Who},
            system::Shutdown,
            Action,
        },
        types::room::Direction,
    },
};

lazy_static! {
    static ref HIGHLIGHT_USAGE: Regex =
        Regex::new(r#"(?P<symbol>[<>\[\]\(\)]|\|\||\.\.)"#).unwrap();
}

pub struct Commands {
    commands: HashMap<&'static str, Command>,
    shortcuts: HashMap<char, &'static str>,
}

impl Commands {
    fn new(initial_commands: Vec<Command>) -> Self {
        let mut commands = HashMap::new();
        let mut shortcuts = HashMap::new();
        for command in initial_commands {
            if let Some(shortcut) = command.shortcut {
                shortcuts.insert(shortcut, command.name);
            }
            commands.insert(command.name, command);
        }
        Commands {
            commands,
            shortcuts,
        }
    }

    pub fn parse(&self, actor: Entity, input: &str, restricted: bool) -> Result<Action, String> {
        if let Some(c) = input.chars().next() {
            if let Some(command) = self.shortcuts.get(&c) {
                let tokenizer = Tokenizer::new(&input[c.len_utf8()..]);
                return self.run_command(command, actor, tokenizer, restricted);
            }
        }

        let mut tokenizer = Tokenizer::new(input);
        if let Some(command) = tokenizer.next() {
            if command == "help" {
                Err(self.help(tokenizer))
            } else {
                self.run_command(command, actor, tokenizer, restricted)
            }
        } else {
            Err("Go on, then.".to_string())
        }
    }

    fn run_command(
        &self,
        name: &str,
        actor: Entity,
        tokenizer: Tokenizer,
        restricted: bool,
    ) -> Result<Action, String> {
        let matches = self
            .commands
            .iter()
            .filter(|(_, command)| !restricted || !command.restricted)
            .map(|(name, _)| name)
            .filter(|n| n.starts_with(name))
            .collect_vec();
        if matches.len() > 1 {
            Err(format!(
                "Be more specific: {} could match.",
                word_list(
                    matches
                        .into_iter()
                        .map(|name| name.to_string())
                        .collect_vec()
                )
            ))
        } else if let Some(key) = self.commands.keys().find(|n| n.starts_with(name)) {
            (self.commands.get(key).unwrap().parser)(actor, tokenizer)
        } else {
            Err("I don't know what that means.".to_string())
        }
    }

    fn help(&self, mut tokenizer: Tokenizer) -> String {
        if tokenizer.rest().is_empty() {
            format!("|SteelBlue3|Welcome to the City Six guidance system.\r\n\r\nGuidance is available on the following topics:|-|\r\n{}", word_list(self.commands.keys().map(|n| format!("|white|{}|-|", n)).collect_vec()))
        } else {
            let topic = tokenizer.next().unwrap();
            if let Some(command) = self.commands.get(&topic) {
                let help = &command.help;
                if let Some(subtopic) = tokenizer.next() {
                    if let Some(subhelp) = help.subhelp.get(subtopic) {
                        subhelp.to_string()
                    } else {
                        format!(
                            "There is no help subtopic of \"{}\" for \"{}.\"",
                            subtopic, topic
                        )
                    }
                } else {
                    help.to_string()
                }
            } else {
                format!("There is no help topic for \"{}.\"", topic)
            }
        }
    }
}

impl Default for Commands {
    fn default() -> Self {
        Commands::new(default_commands())
    }
}

type CommandParser = fn(Entity, Tokenizer) -> Result<Action, String>;

struct Command {
    name: &'static str,
    parser: CommandParser,
    help: Help,
    restricted: bool,
    shortcut: Option<char>,
}

impl Command {
    fn new(name: &'static str, parser: CommandParser, help: Help) -> Self {
        Command {
            name,
            parser,
            help,
            restricted: false,
            shortcut: None,
        }
    }

    fn restricted(mut self) -> Self {
        self.restricted = true;
        self
    }

    fn with_shortcut(mut self, shortcut: char) -> Self {
        self.shortcut = Some(shortcut);
        self
    }
}

struct Help {
    usage: Option<&'static str>,
    example: Option<&'static str>,
    description: &'static str,
    subhelp: HashMap<&'static str, Help>,
}

impl Help {
    fn new(usage: &'static str, description: &'static str) -> Self {
        Help {
            usage: Some(usage),
            example: None,
            description,
            subhelp: HashMap::new(),
        }
    }

    fn new_simple(description: &'static str) -> Self {
        Help {
            usage: None,
            example: None,
            description,
            subhelp: HashMap::new(),
        }
    }

    fn with_example(mut self, example: &'static str) -> Self {
        self.example = Some(example);
        self
    }

    fn with_subhelp(mut self, subcommand: &'static str, help: Help) -> Self {
        self.subhelp.insert(subcommand, help);
        self
    }
}

impl fmt::Display for Help {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(usage) = self.usage {
            let usage = HIGHLIGHT_USAGE.replace_all(usage, UsageColorizer {});
            write!(f, "|white|Usage:|-| {}\r\n\r\n", usage)?;
        }
        write!(f, "{}", self.description)?;
        if let Some(example) = self.example {
            write!(f, "\r\n\r\n|white|Example:|-| {}", example)?;
        }
        if !self.subhelp.is_empty() {
            write!(
                f,
                "\r\n\r\n|white|Subtopics:|-| {}",
                word_list(self.subhelp.keys().map(|k| k.to_string()).collect_vec())
            )?;
        }
        Ok(())
    }
}

struct UsageColorizer {}

impl Replacer for UsageColorizer {
    fn replace_append(&mut self, caps: &regex::Captures<'_>, dst: &mut String) {
        if let Some(symbol) = caps.name("symbol") {
            dst.push_str("|white|");
            dst.push_str(symbol.as_str());
            dst.push_str("|-|")
        }
    }
}

fn default_commands() -> Vec<Command> {
    let mut commands = Vec::new();
    commands.push(Command::new(
        "down",
        |actor, _| {
            Ok(Action::from(Move {
                entity: actor,
                direction: Direction::Down,
            }))
        },
        Help::new("down", "Moves you to the room below, if possible."),
    ));
    commands.push(Command::new(
        "drop",
        parse_drop,
        Help::new(
            "drop <keyword> [<keyword>..]",
            "Drops the item indicated by the specified keyword(s) onto the ground.",
        )
        .with_example("drop fuzzy bear"),
    ));
    commands.push(Command::new(
        "east",
        |actor, _| {
            Ok(Action::from(Move {
                entity: actor,
                direction: Direction::East,
            }))
        },
        Help::new("east", "Moves you to the room to the east, if possible."),
    ));
    commands.push(Command::new(
        "exits",
        |actor, _| Ok(Action::from(Exits { entity: actor })),
        Help::new("exits", "Lists the exits from the current room."),
    ));
    commands.push(Command::new(
        "get",
        parse_get,
        Help::new(
            "get <keyword> [<keyword>..]",
            "Picks up the item indicated by the specified keyword(s) from the ground.",
        )
        .with_example("get fuzzy bear"),
    ));
    commands.push(Command::new(
        "inventory",
        |actor, _| Ok(Action::from(Inventory { entity: actor })),
        Help::new("inventory", "Displays a list of items in your inventory."),
    ));
    commands.push(Command::new(
            "look",
            parse_look,
            Help::new(
                "look [<direction>] || look at <keyword> [<keyword>..]",
                "Look around the room, in an adjacent room, or at something specific in the current room.",
            ).with_example(
                "look west || look at fuzzy bear"
            ),
        ));
    commands.push(Command::new(
            "me",
            parse_me,
            Help::new(
                "me <text> || ;<text>",
                "Causes your character to emote the given text. The example would read: \"Ted dances around.\" for someone named Ted",
            ).with_example("me dances around. || ;dances around."),
        ).with_shortcut(';'));
    commands.push(Command::new(
        "north",
        |actor, _| {
            Ok(Action::from(Move {
                entity: actor,
                direction: Direction::North,
            }))
        },
        Help::new("north", "Moves you to the room to the north, if possible."),
    ));
    commands.push(Command::new(
            "object",
            parse_object,
            Help::new(
                "object new || object <id> <subcommand>",
                "Creates, modifies, and removes objects from the game world."
            ).with_subhelp("new", Help::new("object new", "Creates a new object. New objects are placed in the current room."))
            .with_subhelp("info", Help::new("object <id> info", "Displays information about an object.").with_example("object 2 info")
            .with_subhelp("name", Help::new("object <id> name <text>", "Sets the object's name to <text>. Names should: be nouns, only be capitalized when they are proper nouns, and avoid terminating punctuation.").with_example( "object 2 name fuzzy bear"))
            .with_subhelp("keywords", Help::new("object <id> keywords <keyword> [<keyword>..]", "Sets an object's keywords. Keywords are the primary way players interact with objects and should be obvious from the object's name. Extra keywords can be added for disambiguation.")
            .with_example( "object 2 keywords fuzzy bear")))
            .with_subhelp("desc", Help::new("object <id> desc <text>", "Sets an object's description. Descriptions are prose and should contain one or more complete sentences.")
            .with_example( "object 2 desc An adorable teddy bear. It looks well loved."))
            .with_subhelp("remove", Help::new("object <id> remove", "Removes the object from the game world. This will remove all instances of the object from rooms, players, and other containers.")
            .with_example( "object 2 remove"))
            .with_subhelp("set", Help::new("object <id> set <flag> [<flag>..]", "Sets one or more flags on the object. Use \"help object flags\" for more information about flags.")
            .with_example( "object 2 set fixed subtle"))
            .with_subhelp("unset", Help::new("object <id> unset <flag> [<flag>..]", "Clears one or more flags on the object. Use \"help object flags\" for more information about flags.")
            .with_example( "object 2 unset fixed subtle"))
            .with_subhelp("flags", Help::new_simple("Flags are used to set binary properties on objects.\r\n  |white|fixed|-|: prevents the object from being picked up.\r\n  |white|subtle|-|: prevents the object from being listed in the rooms item list when the look command is used. It can still be looked at, however."))
        ).restricted());
    commands.push(
        Command::new(
            "player",
            parse_player,
            Help::new(
                "player <name> <subcommand>",
                "Commands for managing players in the game world.",
            )
            .with_subhelp(
                "info",
                Help::new("player <name> info", "Displays information about a player.")
                    .with_example("player Ted info"),
            ),
        )
        .restricted(),
    );
    commands.push(Command::new("room", parse_room, Help::new("room <subcommand>", "Creates, modifies, and removes rooms from the game world. All room commands apply to the room you are in (aside from \"room new\").")
        .with_subhelp("info", Help::new("help info", "Displays information about the current room."))
        .with_subhelp("new", Help::new("room new [<direction>]", "Creates a new room. If direction is omitted, the new room will not have any exits and thus not be attached to the world. If a direction is used, the room will be connected in that direction from the current room and a reciprocal exit will be created from the new room to the current room.")
        .with_example("room new west"))
        .with_subhelp("desc", Help::new("room desc <text>", "Sets a room's description. Descriptions are prose and should contain one or more complete sentences.")
        .with_example("room desc This tattoo shop has seen better days. Most corners of the room are grungy, and the chair is torn from wear. A tattoo gun rests on the chair-side table."))
        .with_subhelp("link", Help::new("room link <direction> <destination room ID>", "Adds an exit to the current room in the specified direction which leads to the specified room ID.")
        .with_example("room link down 4"))
        .with_subhelp("unlink", Help::new("room unlink <direction>", "Removes the specified exit from the current room.")
        .with_example("room unlink down"))
        .with_subhelp("regions", Help::new("room regions <region> [<region..>]", "Sets the regions for the current room. Regions are akin to room tags, and can be used to group or categorize rooms.")
        .with_example("room regions city street"))
        .with_subhelp("remove", Help::new("room remove", "Removes the current room and moves its contents to the void room (room 0). This includes all players and objects currently within the room, including the invoking player."))
        ).restricted());
    commands.push(
        Command::new(
            "say",
            parse_say,
            Help::new(
                "say <message> || '<message>",
                "Causes your character to say the specified message to the room.",
            )
            .with_example("say Hello there. || 'Hello there."),
        )
        .with_shortcut('\''),
    );
    commands.push(Command::new("scripts", parse_script, Help::new("script <script name> <subcommand>", "Attaches or detaches a script to or from an object, player, or room.")
        .with_subhelp("attach-pre", Help::new("script <script name> attach-pre (object||player||room) <id/name>", "Attaches the script to the given object, player, or room as a pre-action script. These are processed before the triggering action is executed and can prevent the action from occurring. Objects and rooms are indicated by their ID and players by their name.")
        .with_example("script check_for_keycard attach-pre room 4"))
        .with_subhelp("attach", Help::new("script <script name> attach (object||player||room) <id/name>", "Attaches the script to the given object, player, or room as a post-action script. These are processed after the triggering action has been executed. Objects and rooms are indicated by their ID and players by their name.")
        .with_example("script greet_player attach object 2"))
        .with_subhelp("detach", Help::new("script <script name> detach (object||player||room) <id/name>", "Detaches the script from the given object, player, or room.")
        .with_example("script greet_player detach object 2")))
        .restricted());
    commands.push(Command::new(
        "stats",
        parse_stats,
        Help::new("stats", "Displays your vital statistics."),
    ));
    commands.push(Command::new(
        "send",
        parse_send,
        Help::new(
            "send <recipient> <message>",
            "Sends the specified message to the recipient player.",
        )
        .with_example("send Ted Hello Ted."),
    ));
    commands.push(
        Command::new(
            "shutdown",
            |actor, _| Ok(Action::from(Shutdown { entity: actor })),
            Help::new("shutdown", "Immediately shuts down ReMUD."),
        )
        .restricted(),
    );
    commands.push(Command::new(
        "south",
        |actor, _| {
            Ok(Action::from(Move {
                entity: actor,
                direction: Direction::South,
            }))
        },
        Help::new("south", "Moves you to the room to the south, if possible."),
    ));
    commands.push(
        Command::new(
            "teleport",
            parse_teleport,
            Help::new("teleport <room ID>", "Teleports you to the specified room."),
        )
        .restricted(),
    );
    commands.push(Command::new(
        "up",
        |actor, _| {
            Ok(Action::from(Move {
                entity: actor,
                direction: Direction::Up,
            }))
        },
        Help::new("up", "Moves you to the room above, if possible."),
    ));
    commands.push(Command::new(
        "west",
        |actor, _| {
            Ok(Action::from(Move {
                entity: actor,
                direction: Direction::West,
            }))
        },
        Help::new("west", "Moves you to the room to the west, if possible."),
    ));
    commands.push(Command::new(
        "who",
        |actor, _| Ok(Action::from(Who { entity: actor })),
        Help::new("who", "Retrieves a list of online players."),
    ));
    commands
}
