import { useCallback } from "react";
import { Subscription } from "rxjs";
import { ajax } from "rxjs/ajax";
import { useAuth } from "./use-auth";
import { ScriptAPIResp, Script, CompileError } from "../models/scripts-api";

export const useScriptsApi = (baseURL: string) => {
  const { user } = useAuth();

  const send = useCallback(
    async (script: Script, path: string): Promise<ScriptAPIResp> => {
      return new Promise((resolve, reject) => {
        if (!user || !user.tokens) {
          return reject(new Error("logged in user required"));
        }
        const s: Subscription = ajax({
          url: `${baseURL}/scripts/${path}`,
          method: `POST`,
          body: script,
          timeout: 2000,
          headers: {
            Authorization: `Bearer ${user.tokens.access_token}`,
          },
        }).subscribe({
          next: (r) => {
            s.unsubscribe();
            const resp = r.response as ScriptAPIResp;
            if (resp && resp.error) {
              return reject({ ...resp.error, isSaved: true });
            }
            return resolve(resp);
          },
          error: (err) => {
            s.unsubscribe();
            console.log("err = ", err);
            let reason: CompileError = {
              isSaved: false,
              message: err.message,
            };
            if (err.status === 409) {
              reason.message = `A script with that name already exists.`;
            }
            reject(reason);
          },
          complete: () => s.unsubscribe(),
        });
      });
    },
    [baseURL, user]
  );

  return {
    compile: async (script: Script): Promise<ScriptAPIResp> => {
      return send(script, "compile");
    },
    upsert: async (
      script: Script,
      isCreate: boolean
    ): Promise<ScriptAPIResp> => {
      if (isCreate) {
        return send(script, "create");
      }
      return send(script, "update");
    },
    remove: async (script: Script): Promise<ScriptAPIResp> => {
      return send(script, "delete");
    },
  };
};
