import React, { useState } from "react";
import { Terminal } from "./terminal";
import { SocketContextProvider } from "../services/socket.context";

export const TerminalPage: React.FC = () => {
  const [clicked, setClicked] = useState(false);
  return (
    <SocketContextProvider>
      <div className="flex flex-col justify-between w-full">
        {clicked ? (
          <Terminal />
        ) : (
          <>
            <div className="text-center m-2 mt-20 font-mono italic">
              uplink.city-six.com web console
            </div>
            <div className="text-center mt-10">
              <button
                autoFocus
                className="btn text-3xl font-mono outline-none focus:border-blue-500 animate-pulse"
                onClick={(e) => setClicked(true)}
              >
                Connect
              </button>
            </div>
          </>
        )}
      </div>
    </SocketContextProvider>
  );
};
