import React from "react";
import { AuthStateContext } from "../auth/auth-context";

export const useAuthContext = () => {
  const context = React.useContext(AuthStateContext);
  if (!context) {
    throw new Error("useAuthContext must be used within an AuthProvider");
  }
  return context;
};
