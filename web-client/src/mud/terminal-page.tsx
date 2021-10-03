import React, { useState } from "react";
import { Terminal } from "./terminal";
import { SocketContextProvider } from "../services/socket.context";

export const TerminalPage: React.FC = () => {
  const [clicked, setClicked] = useState(false);
  return (
    <SocketContextProvider>
      <div className="flex flex-col justify-between w-full">
        <div className="text-center m-2 font-mono italic hidden md:block">
          uplink.city-six.com web console
        </div>
        {clicked ? (
          <Terminal />
        ) : (
          <div className="text-center mt-10">
            <button
              className="btn text-3xl font-mono"
              onClick={(e) => setClicked(true)}
            >
              Connect
            </button>
          </div>
        )}
      </div>
    </SocketContextProvider>
  );
};
