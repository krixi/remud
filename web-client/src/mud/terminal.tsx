import React, {
  FormEvent,
  KeyboardEvent,
  useCallback,
  useEffect,
  useReducer,
  useRef,
} from "react";
import { useChat } from "../hooks/use-chat";
import { ConnectionStatus } from "./connection-status";

enum TerminalActionKind {
  HistoryBack,
  HistoryForward,
  Reset,
  Input,
  Submit,
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
  }
  console.warn("UNHANDLED reducer action type: ", action.kind);
  return state;
};

export const Terminal: React.FC = () => {
  const [state, dispatch] = useReducer(reducer, {
    current: 0,
    commands: [],
    input: "",
  });
  const { isConnected, messages, send } = useChat();

  // scroll to the bottom of chat.
  const chatList = useRef(null);
  useEffect(() => {
    if (chatList.current) {
      const chatListElement = chatList.current as HTMLElement;
      chatListElement.scrollTop = chatListElement.scrollHeight;
    }
  }, [messages]);

  const onSubmit = useCallback(
    (e: FormEvent, cmd: string) => {
      e.preventDefault();
      send({ message: cmd });
      dispatch({ kind: TerminalActionKind.Submit, command: cmd });
    },
    [send]
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
    <>
      <div
        className="container bg-black text-gray-69 mx-auto rounded p-5 flex flex-col-reverse items-stretch"
        style={{ height: "90vh" }}
      >
        <div className="flex flex-row-reverse">
          <ConnectionStatus isConnected={isConnected} />
        </div>
        <div ref={chatList} className="overflow-y-auto">
          {messages.map((m, i) => (
            <div
              key={i}
              className="font-mono whitespace-pre-wrap"
              dangerouslySetInnerHTML={{ __html: m.message }}
            />
          ))}
        </div>
      </div>

      <div className="fixed bottom-2 left-0 w-full">
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
                className="w-full p-1 bg-black text-white rounded focus:outline-none"
                autoFocus
                type="text"
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
      </div>
    </>
  );
};

const Prompt: React.FC = () => {
  // from https://heroicons.com/ (MIT)
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      className="h-6 w-6"
      fill="none"
      viewBox="0 0 24 24"
      stroke="currentColor"
    >
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M9 5l7 7-7 7"
      />
    </svg>
  );
};
