import { useCallback, useEffect, useReducer } from "react";
import { useWebSocket } from "../services/socket.context";
import { useObservable } from "./use-observable";
import { Message } from "../models/ws-api";
import { Subscription } from "rxjs";

export interface ChatMessage {
  message: string;
}

interface ChatAction {
  sent: ChatMessage;
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
    const s: Subscription = socket
      ?.on<Message<ChatMessage>>("output")
      .subscribe({
        next: (value) => {
          dispatch({ sent: value.data });
        },
        error: (err) => console.log(" got err ", err),
      });
    return () => s.unsubscribe();
  }, [socket]);

  const send = useCallback(
    (msg: ChatMessage): void => {
      if (socket) {
        socket.emit<ChatMessage>("input", msg); // TODO: event name should be "chat"?
        // TODO: edit the last message in the list to include this input.
        dispatch({ sent: msg });
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
