import React, { useReducer } from "react";
import {
  AuthAction,
  AuthActionKind,
  AuthContext,
  AuthData,
} from "../models/auth-api";

export const AuthStateContext = React.createContext<AuthContext>({
  dispatch: (action) => {},
});

const AuthReducer = (state: AuthData, action: AuthAction) => {
  switch (action.kind) {
    case AuthActionKind.RequestLogin:
      return {
        ...state,
        loading: true,
      };
    case AuthActionKind.LoginSuccess:
      return {
        ...state,
        loading: false,
        tokens: action.tokens,
      };
    case AuthActionKind.Logout:
      return {
        ...state,
        tokens: undefined,
      };
    case AuthActionKind.LoginError:
      return {
        ...state,
        tokens: undefined,
        loading: false,
      };
  }
};

export const AuthProvider: React.FC = ({ children }) => {
  const [state, dispatch] = useReducer(AuthReducer, {
    tokens: undefined,
  });

  return (
    <AuthStateContext.Provider
      value={{
        data: state,
        dispatch,
      }}
    >
      {children}
    </AuthStateContext.Provider>
  );
};
