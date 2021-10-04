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
  const { isConnected, isSensitivePrompt, messages, send } = useChat();

  // scroll to the bottom of chat.
  const chatList = useRef(null);
  useEffect(() => {
    if (chatList.current) {
      const chatListElement = chatList.current as HTMLElement;
      chatListElement.scrollTop = chatListElement.scrollHeight;
    }
  }, [messages]);

  // render assembles a JSX element from a chat line.
  const render = useCallback((m: ChatLine): JSX.Element => {
    // This supports nested colors by maintaining a color stack
    let colors: string[] = [];
    const wrapInColor = (msg: JSX.Element): JSX.Element => {
      const color = colors.pop();
      return color ? <span style={{ color: `#${color}` }}>{msg}</span> : msg;
    };

    // Assemble the segments into a line
    let msg = <></>;
    let current = 0;
    while (current < m.segments.length) {
      const segment = m.segments[current];
      if (isMessage(segment)) {
        msg = (
          <>
            {msg}
            {getMessage(segment)}
          </>
        );
      } else if (isColorStart(segment)) {
        colors.push(getColor(segment));
      } else if (isColorEnd(segment)) {
        msg = wrapInColor(msg);
      }
      current++;
    }
    while (colors.length > 0) {
      msg = wrapInColor(msg);
    }

    return msg;
  }, []);

  return (
    <>
      <div className="container bg-black text-gray-69 mx-auto mt-1 rounded p-5 flex flex-col-reverse items-stretch vh-85 md:vh-90">
        <div className="flex flex-row-reverse">
          <ConnectionStatus isConnected={isConnected} />
        </div>
        <div
          ref={chatList}
          className="overflow-y-auto font-mono whitespace-pre-wrap"
        >
          {messages.map((m, i) => (
            <div key={i}>{render(m)}</div>
          ))}
        </div>
      </div>

      <div className="fixed bottom-2 left-0 w-full">
        <TerminalInput send={send} isSensitivePrompt={isSensitivePrompt} />
      </div>
    </>
  );
};
