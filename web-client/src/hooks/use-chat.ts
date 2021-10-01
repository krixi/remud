import { useCallback, useEffect, useReducer } from "react";
import { useWebSocket } from "../services/socket.context";
import { useObservable } from "./use-observable";
import { Message } from "../models/ws-api";
import { Subscription } from "rxjs";

export interface ChatMessage {
  message: string;
}

enum ChatActionKind {
  Connected,
  Disconnected,
  ReceivedMsg,
}

interface ChatAction {
  kind: ChatActionKind;
  sent?: ChatMessage;
  connected?: boolean;
}
interface ChatState {
  messages: ChatMessage[];
}

export interface Chat {
  messages: ChatMessage[];
  isConnected: boolean;
  send: (msg: ChatMessage) => void;
}

const reducer = (state: ChatState, action: ChatAction): ChatState => {
  if (state.messages.length > 250) {
    state.messages.shift();
  }
  return {
    ...state,
    messages: [...state.messages, action.sent!],
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
    const s: Subscription = socket?.on<Message<ChatMessage>>("chat").subscribe({
      next: (value) => {
        dispatch({
          kind: ChatActionKind.ReceivedMsg,
          sent: value.data as ChatMessage,
        });
      },
      error: (err) => console.log(" got err ", err),
    });
    return () => s.unsubscribe();
  }, [socket]);

  const send = useCallback(
    (msg: ChatMessage): void => {
      if (socket) {
        socket.emit<ChatMessage>("input", msg); // TODO: event name should be "chat"?
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
