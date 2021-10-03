import { useCallback, useEffect, useReducer } from "react";
import { useWebSocket } from "../services/socket.context";
import { useObservable } from "./use-observable";
import { Message } from "../models/ws-api";
import { Subscription } from "rxjs";

export interface ChatSent {
  text: string;
}
export const isMessage = (segment: MessageSegment): boolean => {
  return segment.t === "m";
};
export const getMessage = (segment: MessageSegment): string => {
  return isMessage(segment) ? (segment.d as ChatSent).text : "";
};

export interface ChatColorStart {
  color: string;
}
export const isColorStart = (segment: MessageSegment): boolean => {
  return segment.t === "cs";
};
export const getColor = (segment: MessageSegment): string => {
  return isColorStart(segment) ? (segment.d as ChatColorStart).color : "";
};

export interface ChatColorEnd {}
export const isColorEnd = (segment: MessageSegment): boolean => {
  return segment.t === "ce";
};

export interface MessageSegment {
  t: string;
  d: ChatSent | ChatColorStart | ChatColorEnd;
}
export interface ChatLine {
  segments?: MessageSegment[];
  prompt?: boolean;
  message?: string;
}

interface ChatAction {
  append?: boolean;
  sent: ChatLine;
}
interface ChatState {
  messages: ChatLine[];
}

export interface Chat {
  messages: ChatLine[];
  isConnected: boolean;
  send: (msg: ChatLine) => void;
}

const reducer = (state: ChatState, action: ChatAction): ChatState => {
  // only keep recent history
  if (state.messages.length > 250) {
    state.messages.shift();
  }
  return {
    ...state,
    messages: [...state.messages, action.sent],
  };
};

export const useChat = (): Chat => {
  const [state, dispatch] = useReducer(reducer, {
    messages: [],
  });
  const socket = useWebSocket();
  const isConnected = useObservable(socket?.connectionStatus);

  useEffect(() => {
    if (!socket) {
      return;
    }
    const s: Subscription = socket?.on<Message<ChatLine>>("game").subscribe({
      next: (value) => {
        dispatch({ sent: value.data });
      },
      error: (err) => console.log(" got err ", err),
    });
    return () => s.unsubscribe();
  }, [socket]);

  const send = useCallback(
    (msg: ChatLine): void => {
      if (socket) {
        socket.emit<ChatLine>("game", msg);
        dispatch({ sent: msg, append: true });
      }
    },
    [socket]
  );

  return {
    messages: state.messages,
    isConnected: !!isConnected,
    send,
  };
};
