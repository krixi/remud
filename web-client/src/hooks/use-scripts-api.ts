import { useCallback } from "react";
import { Subscription } from "rxjs";
import { ajax, AjaxResponse } from "rxjs/ajax";
import { useAuth } from "./use-auth";
import {
  ScriptAPIResp,
  Script,
  CompileError,
  ListScriptsResp,
  GetScriptReq,
  ListScriptsReq,
  ScriptInfo,
} from "../models/scripts-api";

export const useScriptsApi = (baseURL: string) => {
  const { user, refresh } = useAuth();

  const refreshIfNeeded = useCallback(
    async (
      req: (access_token: string) => Promise<AjaxResponse<any>>
    ): Promise<AjaxResponse<any>> => {
      return new Promise<AjaxResponse<any>>((resolve, reject) => {
        if (!user?.expires) {
          return reject(new Error("logged in user required"));
        }

        // if we are within 1 minute of the expiration, fetch a new access token before continuing.
        const soon = new Date();
        soon.setMinutes(soon.getMinutes() + 1);
        if (user.expires <= soon || !user.tokens?.access_token) {
          // refresh the token before letting the base request through
          refresh()
            .then((tokens) => resolve(req(tokens.access_token)))
            .catch((err) => reject(err));
        } else {
          // no need to refresh - just let it go
          return resolve(req(user.tokens.access_token));
        }
      });
    },
    [refresh, user?.expires, user?.tokens?.access_token]
  );

  const send = useCallback(
    async (
      body: Script | GetScriptReq | ListScriptsReq,
      path: string
    ): Promise<AjaxResponse<any>> => {
      // we wrap this promise in a function so that we can pass in the correct access token after it's been refreshed.
      const sendReq = (access_token: string) =>
        new Promise<AjaxResponse<any>>((resolve, reject) => {
          const s: Subscription = ajax({
            url: `${baseURL}/scripts/${path}`,
            method: `POST`,
            body,
            timeout: 2000,
            headers: {
              Authorization: `Bearer ${access_token}`,
            },
          }).subscribe({
            next: (r) => {
              s.unsubscribe();
              return resolve(r);
            },
            error: (err) => {
              s.unsubscribe();
              reject(err);
            },
            complete: () => s.unsubscribe(),
          });
        });
      return refreshIfNeeded(sendReq);
    },
    [baseURL, refreshIfNeeded]
  );

  const checkForErr = useCallback(
    async (req: Promise<AjaxResponse<any>>): Promise<CompileError | void> => {
      return new Promise<CompileError | void>((resolve, reject) => {
        req
          .then((r) => {
            const resp = r.response as ScriptAPIResp;
            if (resp && resp.error) {
              return reject({ ...resp.error, isSaved: true });
            }
            return resolve();
          })
          .catch((err) => {
            let reason: CompileError = {
              isSaved: false,
              message: err.message,
            };
            if (err.status === 409) {
              reason.message = `A script with that name already exists.`;
            }
            return reject(reason);
          });
      });
    },
    []
  );

  const compile = useCallback(async (script: Script): Promise<CompileError> => {
    //return send(script, "compile");
    return Promise.reject("not implemented"); // TODO
  }, []);

  const get = useCallback(
    async (name: string): Promise<Script> => {
      return new Promise<Script>((resolve, reject) => {
        send({ name }, "read")
          .then((r) => resolve(r.response as Script))
          .catch((err) => reject(err));
      });
    },
    [send]
  );

  const list = useCallback(async (): Promise<ScriptInfo[]> => {
    return new Promise<ScriptInfo[]>((resolve, reject) => {
      send({}, "read/all")
        .then((r) => {
          const resp = r.response as ListScriptsResp;
          return resolve(resp.scripts);
        })
        .catch((err) => reject(err));
    });
  }, [send]);

  const remove = useCallback(
    async (script: Script): Promise<void> => {
      return new Promise<void>((resolve, reject) => {
        send(script, "delete")
          .then(() => resolve())
          .catch((err) => reject(err));
      });
    },
    [send]
  );

  const upsert = useCallback(
    async (script: Script, isCreate: boolean): Promise<CompileError | void> => {
      if (isCreate) {
        return checkForErr(send(script, "create"));
      }
      return checkForErr(send(script, "update"));
    },
    [send, checkForErr]
  );

  return {
    compile,
    get,
    list,
    remove,
    upsert,
  };
};
