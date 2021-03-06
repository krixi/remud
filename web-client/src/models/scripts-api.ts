export enum Trigger {
  Drop = "Drop",
  Emote = "Emote",
  Exits = "Exits",
  Get = "Get",
  Init = "Init",
  Inventory = "Inventory",
  Look = "Look",
  LookAt = "LookAt",
  Move = "Move",
  Say = "Say",
  Send = "Send",
  Timer = "Timer",
  Use = "Use",
}

export interface Script {
  name: string;
  trigger: Trigger;
  code: string;
  error?: CompileError;
}

export interface ScriptInfo {
  name: string;
  trigger?: Trigger;
  lines?: number;
  error?: CompileError;
}

export interface ListScriptsReq {}
export interface ListScriptsResp {
  scripts: ScriptInfo[];
}

export interface GetScriptReq {
  name: string;
}

export interface CompileError {
  isSaved?: boolean;
  line?: number;
  position?: number;
  message: string;
}

// for both save and compile checks
export interface ScriptAPIResp {
  error?: CompileError;
}
