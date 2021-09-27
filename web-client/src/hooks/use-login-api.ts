import { useCallback } from "react";
import { Subscription, timeout } from "rxjs";
import { ajax } from "rxjs/ajax";
import { useAuthContext } from "./use-auth-context";
import { AuthActionKind, AuthTokens, LoginReq } from "../models/auth-api";

export const useLoginApi = (baseURL: string) => {
  const { dispatch } = useAuthContext();

  return useCallback(
    async (req: LoginReq): Promise<void> => {
      dispatch({ kind: AuthActionKind.RequestLogin });
      return new Promise((resolve, reject) => {
        const s: Subscription = ajax
          .post(`${baseURL}/auth/login`, req)
          .pipe(timeout(2000))
          .subscribe({
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
};
