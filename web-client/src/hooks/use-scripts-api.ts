import { ScriptAPIResp, Script, CompileError } from "../models/scripts-api";
import { Subscription, timeout } from "rxjs";
import { ajax } from "rxjs/ajax";
import { useCallback } from "react";

export const useScriptsApi = (baseURL: string) => {
  const send = useCallback(
    async (script: Script, path: string): Promise<ScriptAPIResp> => {
      return new Promise((resolve, reject) => {
        const s: Subscription = ajax
          .post(`${baseURL}/scripts/${path}`, script)
          .pipe(timeout(2000))
          .subscribe({
            next: (r) => {
              s.unsubscribe();
              const resp = r.response as ScriptAPIResp;
              if (resp.error !== undefined) {
                return reject({ ...resp.error, isSaved: true });
              }
              return resolve(resp);
            },
            error: (err) => {
              s.unsubscribe();
              let reason: CompileError = {
                isSaved: false,
                message: err.message,
              };
              console.log("err = ", err);
              if (err.status === 409) {
                reason.message = `A script with that name already exists.`;
              }
              reject(reason);
            },
            complete: () => s.unsubscribe(),
          });
      });
    },
    [baseURL]
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
