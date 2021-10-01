import React, { FormEvent, useCallback, useRef, useState } from "react";
import { useChat } from "../hooks/use-chat";
import { ConnectionStatus } from "./connection-status";

export const Terminal: React.FC = () => {
  const { isConnected, messages, send } = useChat();
  const [command, setCommand] = useState("");

  // scroll to the bottom of chat.
  const chatList = useRef(null);
  if (chatList.current) {
    const chatListElement = chatList.current as HTMLElement;
    chatListElement.scrollTop = chatListElement.scrollHeight;
  }

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
            <pre key={i}>{m.message}</pre>
          ))}
        </div>
      </div>
      <form
        className="container max-w-6xl mx-auto text-black rounded m-2"
        onSubmit={(e) => onSubmit(e, command)}
      >
        <div className="flex flex-row justify-between">
          <input
            className="w-3/4 p-1"
            type="text"
            value={command}
            onChange={(e) => setCommand(e.target.value)}
          />
          <input className="w-1/4 p-1 ml-1" type="submit" value={"Send"} />
        </div>
      </form>
    </>
  );
};
