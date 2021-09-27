import { useCallback, useMemo } from "react";
import { Subscription } from "rxjs";
import { ajax } from "rxjs/ajax";
import { useAuthContext } from "./use-auth-context";
import { Auth, AuthActionKind, AuthTokens, LoginReq } from "../models/auth-api";
import { ScriptApiBaseUrl } from "../env";

export const useAuth = (): Auth => {
  const { data, dispatch } = useAuthContext();
  const baseURL = useMemo(() => {
    return ScriptApiBaseUrl();
  }, []);

  const login = useCallback(
    async (req: LoginReq): Promise<void> => {
      dispatch({ kind: AuthActionKind.RequestLogin });
      return new Promise((resolve, reject) => {
        const s: Subscription = ajax({
          url: `${baseURL}/auth/login`,
          method: `POST`,
          body: req,
          timeout: 2000,
          headers: {
            "Content-Type": "application/json",
          },
        }).subscribe({
          next: (r) => {
            s.unsubscribe();
            const tokens = r.response as AuthTokens;
            dispatch({ kind: AuthActionKind.LoginSuccess, tokens });
            return resolve();
          },
          error: (err) => {
            s.unsubscribe();
            dispatch({ kind: AuthActionKind.LoginError });
            return reject(err);
          },
          complete: () => s.unsubscribe(),
        });
      });
    },
    [baseURL, dispatch]
  );

  const logout = useCallback(async (): Promise<void> => {
    return new Promise((resolve, reject) => {
      // already logged out?
      if (!data || !data.tokens) {
        return resolve();
      }
      const s: Subscription = ajax({
        url: `${baseURL}/auth/logout`,
        method: `POST`,
        body: {},
        timeout: 2000,
        headers: {
          Authorization: `Bearer ${data.tokens.access_token}`,
        },
      }).subscribe({
        next: (val) => {
          s.unsubscribe();
          dispatch({ kind: AuthActionKind.Logout });
          return resolve();
        },
        error: (err) => {
          s.unsubscribe();
          dispatch({ kind: AuthActionKind.Logout });
          return reject(err);
        },
        complete: () => {
          s.unsubscribe();
        },
      });
    });
  }, [baseURL, dispatch, data]);

  const isLoggedIn = useMemo(() => data?.tokens !== undefined, [data]);

  return {
    isLoggedIn,
    user: data,
    login,
    logout,
  };
};
