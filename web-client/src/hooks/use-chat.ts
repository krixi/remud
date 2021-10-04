import { useCallback, useEffect, useReducer } from "react";
import { useWebSocket } from "../services/socket.context";
import { useObservable } from "./use-observable";
import { Message } from "../models/ws-api";
import { Subscription } from "rxjs";

interface ChatSent {
  text: string;
}
export const isMessage = (segment: MessageSegment): boolean => {
  return segment.t === "t";
};
export const getMessage = (segment: MessageSegment): string => {
  return isMessage(segment) ? (segment.d as ChatSent).text : "";
};

interface ChatColorStart {
  color: string;
}
export const isColorStart = (segment: MessageSegment): boolean => {
  return segment.t === "cs";
};
export const getColor = (segment: MessageSegment): string => {
  return isColorStart(segment) ? (segment.d as ChatColorStart).color : "";
};

interface ChatColorEnd {}
export const isColorEnd = (segment: MessageSegment): boolean => {
  return segment.t === "ce";
};

export interface MessageSegment {
  t: string;
  d: ChatSent | ChatColorStart | ChatColorEnd;
}
export interface ChatLine {
  segments: MessageSegment[];
  is_prompt?: boolean;
  is_sensitive?: boolean;
  is_updated?: boolean; // whether or not this is a prompt that's been updated by the client
}

export interface PlayerMessage {
  message: string;
}

interface ChatAction {
  from_server?: ChatLine;
  from_client?: string;
}
interface ChatState {
  messages: ChatLine[];
  isSensitivePrompt: boolean;
}

export interface Chat {
  messages: ChatLine[];
  isConnected: boolean;
  isSensitivePrompt: boolean;
  send: (msg: string) => void;
}

const reducer = (state: ChatState, action: ChatAction): ChatState => {
  // only keep recent history
  if (state.messages.length > 250) {
    state.messages.shift();
  }
  if (action.from_server) {
    return {
      ...state,
      isSensitivePrompt: !!(
        action.from_server.is_prompt && action.from_server.is_sensitive
      ),
      messages: [...state.messages, action.from_server],
    };
  } else if (action.from_client) {
    // append the input to the last prompt.
    for (let idx = state.messages.length - 1; idx >= 0; idx--) {
      if (state.messages[idx].is_prompt) {
        if (!state.messages[idx].is_sensitive) {
          state.messages[idx].segments!.push({
            t: "t",
            d: {
              text: action.from_client,
            },
          });
          state.messages[idx].is_updated = true;
        }
        break;
      }
    }
    return {
      ...state,
      messages: [...state.messages],
    };
  }
  return state;
};

export const useChat = (): Chat => {
  const [state, dispatch] = useReducer(reducer, {
    messages: [],
    isSensitivePrompt: false,
  });
  const socket = useWebSocket();
  const isConnected = useObservable(socket?.connectionStatus);

  useEffect(() => {
    if (!socket) {
      return;
    }
    const s: Subscription = socket?.on<Message<ChatLine>>("game").subscribe({
      next: (value) => {
        dispatch({ from_server: value.data });
      },
      error: (err) => console.log(" got err ", err),
    });
    return () => s.unsubscribe();
  }, [socket]);

  const send = useCallback(
    (msg: string): void => {
      if (socket) {
        socket.emit<PlayerMessage>("game", { message: msg });
        dispatch({
          from_client: msg,
        });
      }
    },
    [socket]
  );

  return {
    messages: state.messages,
    isConnected: !!isConnected,
    isSensitivePrompt: state.isSensitivePrompt,
    send,
  };
};
