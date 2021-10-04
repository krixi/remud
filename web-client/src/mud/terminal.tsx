import React, { useCallback, useEffect, useMemo, useRef } from "react";
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
import { BehaviorSubject } from "rxjs";

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
    let result: JSX.Element[] = [];
    let colors: string[] = []; // stack of colors used
    const wrapInColor = (msg: string): JSX.Element => {
      const color = colors.pop();
      return color ? (
        <span style={{ color: `#${color}` }}>{msg}</span>
      ) : (
        <>{msg}</>
      );
    };

    let current = 0;
    let phrase = "";
    while (current < m.segments.length) {
      const segment = m.segments[current];
      if (isMessage(segment)) {
        phrase = `${phrase}${getMessage(segment)}`;
      } else if (isColorStart(segment)) {
        // const m = msg[msg.length-1];
        result.push(<>{phrase}</>);
        colors.push(getColor(segment));
        phrase = "";
      } else if (isColorEnd(segment)) {
        result.push(wrapInColor(phrase));
        phrase = "";
      }
      current++;
    }
    if (phrase) {
      result.push(wrapInColor(phrase));
    }

    return <>{result.map((m) => m)}</>;
  }, []);

  const focusListener = useMemo(() => {
    return new BehaviorSubject<boolean>(false);
  }, []);
  const focusComplete = useCallback(() => {
    focusListener.next(false);
  }, [focusListener]);

  return (
    <>
      <div
        className="container bg-black text-gray-69 mx-auto mt-1 rounded p-5 flex flex-col-reverse items-stretch vh-85 md:vh-90"
        onClick={(e) => {
          e.preventDefault();
          focusListener.next(true);
        }}
      >
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
        <TerminalInput
          send={send}
          isSensitivePrompt={isSensitivePrompt}
          refocus={focusListener}
          focusComplete={focusComplete}
        />
      </div>
    </>
  );
};
