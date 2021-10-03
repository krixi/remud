import React, {
  createContext,
  useContext,
  useEffect,
  useMemo,
  useState,
} from "react";
import { NewSocketService, SocketService } from "./socket.service";
import { WebsocketBaseUrl } from "../env";

const SocketContext = createContext<SocketService | undefined>(undefined);

export const SocketContextProvider: React.FC = ({ children }) => {
  const [socket, setSocket] = useState<SocketService | undefined>();

  const uri = useMemo(() => {
    return WebsocketBaseUrl();
  }, []);

  useEffect(() => {
    setSocket(NewSocketService(uri));
  }, [uri]);

  return (
    <SocketContext.Provider value={socket}>{children}</SocketContext.Provider>
  );
};

export const useWebSocket = (): SocketService | undefined =>
  useContext(SocketContext);
