import React, { useCallback, useEffect, useRef } from "react";
import {
  ChatLine,
  getColor,
  getMessage,
  isColorEnd,
  isColorStart,
  isMessage,
  useChat,
} from "../hooks/use-chat";
import { ConnectionStatus } from "./connection-status";
import { TerminalInput } from "./terminal-input";

export const Terminal: React.FC = () => {
  const { isConnected, messages, send } = useChat();

  // scroll to the bottom of chat.
  const chatList = useRef(null);
  useEffect(() => {
    if (chatList.current) {
      const chatListElement = chatList.current as HTMLElement;
      chatListElement.scrollTop = chatListElement.scrollHeight;
    }
  }, [messages]);

  // render is a recursive function that assembles a JSX element from a chat line.
  const render = useCallback(
    (m: ChatLine, current?: number, msg?: JSX.Element): JSX.Element => {
      if (msg === undefined) {
        msg = <></>;
      }
      if (!m.segments) {
        return msg;
      }
      if (current === undefined) {
        current = 0;
      } else if (current >= m.segments.length) {
        return msg;
      }

      let segment = m.segments[current];
      if (isMessage(segment)) {
        return render(m, current + 1, <>{getMessage(segment)}</>);
      } else if (isColorStart(segment)) {
        return (
          <span style={{ color: `#${getColor(segment)}` }}>
            {render(m, current + 1, msg)}
          </span>
        );
      } else if (isColorEnd(segment)) {
        return msg;
      }

      return msg;
    },
    []
  );

  return (
    <>
      <div className="container bg-black text-gray-69 mx-auto mt-1 rounded p-5 flex flex-col-reverse items-stretch vh-85 md:vh-90">
        <div className="flex flex-row-reverse">
          <ConnectionStatus isConnected={isConnected} />
        </div>
        <div ref={chatList} className="overflow-y-auto">
          {messages.map((m, i) => (
            <div
              key={i}
              className="font-mono whitespace-pre-wrap"
              dangerouslySetInnerHTML={{ __html: m.message! }}
            />
          ))}
        </div>
      </div>

      <div className="fixed bottom-2 left-0 w-full">
        <TerminalInput send={send} />
      </div>
    </>
  );
};
