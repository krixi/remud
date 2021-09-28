import jwt_decode from "jwt-decode";
import React, { useEffect, useMemo, useReducer } from "react";
import {
  AuthAction,
  AuthActionKind,
  AuthContext,
  AuthTokens,
  DecodedToken,
  UserData,
} from "../models/auth-api";

export const AuthStateContext = React.createContext<AuthContext>({
  dispatch: (action) => {},
});

const decode = (tokens?: AuthTokens): UserData => {
  // decode the access token to get more useful data
  let name = "",
    expires = undefined,
    scopes: string[] = [];
  if (tokens?.access_token) {
    const t = jwt_decode<DecodedToken>(tokens?.access_token);
    name = t.sub || "";
    expires = t.exp ? new Date(t.exp * 1000) : undefined;
    scopes = t.scopes;
  }
  return {
    tokens,
    name,
    expires,
    scopes,
  };
};

const AuthReducer = (state: UserData, action: AuthAction): UserData => {
  switch (action.kind) {
    case AuthActionKind.RequestLogin:
      return {
        ...state,
        loading: true,
      };
    case AuthActionKind.RefreshSuccess: // fallthrough
    case AuthActionKind.LoginSuccess:
      return {
        ...state,
        ...decode(action.tokens),
        loading: false,
        refreshPending: false,
      };
    case AuthActionKind.LoginError: // fallthrough
    case AuthActionKind.RefreshError: // fallthrough
    case AuthActionKind.Logout:
      // reset the whole state
      return {};
  }
};

export const AuthProvider: React.FC = ({ children }) => {
  // Save the state in local storage so it persists through refreshes.
  const initialState: UserData = useMemo(() => {
    const access_token = sessionStorage.getItem("access_token") || "";
    const refresh_token = localStorage.getItem("refresh_token") || "";
    return {
      ...decode({
        access_token,
        refresh_token,
      }),
      refreshPending: true,
    };
  }, []);
  const [data, dispatch] = useReducer(AuthReducer, initialState);

  // Store the logged in user in local storage
  useEffect(() => {
    sessionStorage.setItem("access_token", data?.tokens?.access_token || "");
    localStorage.setItem("refresh_token", data?.tokens?.refresh_token || "");
  }, [data]);

  return (
    <AuthStateContext.Provider
      value={{
        data,
        dispatch,
      }}
    >
      {children}
    </AuthStateContext.Provider>
  );
};
