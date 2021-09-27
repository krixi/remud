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
  const { user } = useAuth();

  const send = useCallback(
    async (
      body: Script | GetScriptReq | ListScriptsReq,
      path: string
    ): Promise<AjaxResponse<any>> => {
      // TODO: if user.accessTokenExpires expires soon, first fetch a new access token.
      // or, respond to a 401 with header www-authenticate or some shit

      return new Promise((resolve, reject) => {
        if (!user || !user.tokens) {
          return reject(new Error("logged in user required"));
        }
        const s: Subscription = ajax({
          url: `${baseURL}/scripts/${path}`,
          method: `POST`,
          body,
          timeout: 2000,
          headers: {
            Authorization: `Bearer ${user.tokens.access_token}`,
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
    },
    [baseURL, user]
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
