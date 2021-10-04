import React, {
  FormEvent,
  KeyboardEvent,
  useCallback,
  useEffect,
  useReducer,
  useRef,
} from "react";
import { Prompt } from "./prompt";
import { Observable } from "rxjs";
import { useObservable } from "../hooks/use-observable";

enum TerminalActionKind {
  HistoryBack,
  HistoryForward,
  Reset,
  Input,
  Submit,
  SubmitSensitive,
}

interface TerminalAction {
  kind: TerminalActionKind;
  command?: string;
}

interface TerminalState {
  commands: string[];
  current: number;
  input: string;
}

const reducer = (
  state: TerminalState,
  action: TerminalAction
): TerminalState => {
  // only keep recent history
  if (state.commands.length > 250) {
    state.commands.shift();
  }
  let current = state.current;
  switch (action.kind) {
    case TerminalActionKind.HistoryForward:
      current =
        state.current < state.commands.length
          ? state.current + 1
          : state.commands.length;
      return {
        ...state,
        current,
        input: state.commands[current] ? `${state.commands[current]}` : "",
      };
    case TerminalActionKind.HistoryBack:
      current = state.current > 0 ? state.current - 1 : 0;
      return {
        ...state,
        current,
        input: state.commands[current] ? `${state.commands[current]}` : "",
      };
    case TerminalActionKind.Reset:
      return {
        ...state,
        current: state.commands.length,
        input: "",
      };
    case TerminalActionKind.Input:
      return {
        ...state,
        input: action.command!,
      };
    case TerminalActionKind.Submit:
      return {
        ...state,
        current: state.current + 1,
        commands: [...state.commands, action.command!],
        input: "",
      };
    case TerminalActionKind.SubmitSensitive:
      return {
        ...state,
        input: "",
      };
  }
  console.warn("UNHANDLED reducer action type: ", action.kind);
  return state;
};

export interface PublicProps {
  send: (msg: string) => void;
  isSensitivePrompt: boolean;
  refocus: Observable<boolean>;
  focusComplete: () => void;
}

export const TerminalInput: React.FC<PublicProps> = ({
  send,
  isSensitivePrompt,
  refocus,
  focusComplete,
}) => {
  const [state, dispatch] = useReducer(reducer, {
    current: 0,
    commands: [],
    input: "",
  });
  const shouldFocus = useObservable<boolean>(refocus);
  const inputRef = useRef(null);

  useEffect(() => {
    if (shouldFocus && inputRef.current) {
      const ele = inputRef.current as HTMLElement;
      ele.focus();
      focusComplete();
    }
  }, [shouldFocus, focusComplete]);

  const onSubmit = useCallback(
    (e: FormEvent, cmd: string) => {
      e.preventDefault();
      send(cmd);
      dispatch({
        kind: isSensitivePrompt
          ? TerminalActionKind.SubmitSensitive
          : TerminalActionKind.Submit,
        command: cmd,
      });
    },
    [send, isSensitivePrompt]
  );

  const onKeyPressed = useCallback((e: KeyboardEvent) => {
    if (e.key === "ArrowUp") {
      e.preventDefault();
      dispatch({ kind: TerminalActionKind.HistoryBack });
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      dispatch({ kind: TerminalActionKind.HistoryForward });
    } else if (e.key === "Escape") {
      e.preventDefault();
      dispatch({ kind: TerminalActionKind.Reset });
    }
  }, []);

  return (
    <form
      className="container mt-1 mx-auto"
      onSubmit={(e) => onSubmit(e, state.input)}
    >
      <div className="flex flex-row justify-between">
        <div className="w-3/4 flex flex-row text-white bg-black rounded">
          <div className="font-mono p-2">
            <Prompt />
          </div>
          <input
            ref={inputRef}
            className="w-full p-1 bg-black text-white rounded focus:outline-none"
            autoFocus
            type={isSensitivePrompt ? "password" : "text"}
            value={state.input}
            onChange={(e) =>
              dispatch({
                kind: TerminalActionKind.Input,
                command: e.target.value,
              })
            }
            onKeyDown={(e) => onKeyPressed(e)}
          />
        </div>
        <input
          className="w-1/4 p-1 mx-1 cursor-pointer bg-soft-gray btn"
          type="submit"
          value={"Send"}
        />
      </div>
    </form>
  );
};
