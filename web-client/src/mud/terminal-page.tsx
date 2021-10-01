import React from "react";
import { Terminal } from "./terminal";
import { SocketContextProvider } from "../services/socket.context";

export const TerminalPage: React.FC = () => {
  return (
    <SocketContextProvider>
      <div className="w-full">
        <div className="text-center m-2 font-mono italic">
          uplink.city-six.com web console
        </div>
        <Terminal />
      </div>
    </SocketContextProvider>
  );
};
