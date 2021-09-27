import jwt_decode from "jwt-decode";
import React, { useReducer } from "react";
import {
  AuthAction,
  AuthActionKind,
  AuthContext,
  DecodedToken,
  UserData,
} from "../models/auth-api";

export const AuthStateContext = React.createContext<AuthContext>({
  dispatch: (action) => {},
});

const AuthReducer = (state: UserData, action: AuthAction) => {
  switch (action.kind) {
    case AuthActionKind.RequestLogin:
      return {
        ...state,
        loading: true,
      };
    case AuthActionKind.LoginSuccess:
      // decode the access token to get more useful data
      let name = "",
        expires = undefined,
        scopes: string[] = [];
      if (action.tokens?.access_token) {
        const t = jwt_decode<DecodedToken>(action.tokens?.access_token);
        name = t.sub || "";
        expires = t.exp ? new Date(t.exp * 1000) : undefined;
        scopes = t.scopes;
      }
      return {
        ...state,
        loading: false,
        tokens: action.tokens,
        name,
        expires,
        scopes,
      };
    case AuthActionKind.Logout:
      // reset the whole state
      return {};
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
