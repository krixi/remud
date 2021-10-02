import React, {
  FormEvent,
  useCallback,
  useEffect,
  useRef,
  useState,
} from "react";
import { useChat } from "../hooks/use-chat";
import { ConnectionStatus } from "./connection-status";

export const Terminal: React.FC = () => {
  const { isConnected, messages, send } = useChat();
  const [command, setCommand] = useState("");

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
      setCommand("");
    },
    [send]
  );

  return (
    <>
      <div
        className="container max-w-6xl bg-black text-white mx-auto rounded p-5"
        style={{ height: "80vh" }}
      >
        <div className="flex flex-row-reverse">
          <ConnectionStatus isConnected={isConnected} />
        </div>
        <div
          ref={chatList}
          className="overflow-auto"
          style={{ height: "75vh" }}
        >
          {messages.map((m, i) => (
            <div
              key={i}
              className="font-mono whitespace-pre-wrap"
              dangerouslySetInnerHTML={{ __html: m.message }}
            />
          ))}
        </div>
      </div>
      <form
        className="container max-w-6xl m-2 mx-auto"
        onSubmit={(e) => onSubmit(e, command)}
      >
        <div className="flex flex-row justify-between">
          <div className="w-3/4 flex flex-row text-white bg-black rounded mx-1">
            <div className="font-mono p-2">
              <Prompt />
            </div>
            <input
              className="w-full p-1 bg-black text-white rounded focus:outline-none"
              autoFocus
              type="text"
              value={command}
              onChange={(e) => setCommand(e.target.value)}
            />
          </div>
          <input
            className="w-1/4 p-1 mx-1 cursor-pointer bg-soft-gray btn"
            type="submit"
            value={"Send"}
          />
        </div>
      </form>
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
