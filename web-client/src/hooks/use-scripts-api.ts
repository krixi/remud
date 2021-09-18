import { ScriptAPIResp, Script } from "../models/scripts-api";
import { firstValueFrom, timeout } from "rxjs";
import { ajax } from "rxjs/ajax";
import { useCallback } from "react";

export const useScriptsApi = (baseURL: string) => {
  const send = useCallback(
    async (script: Script, path: string): Promise<ScriptAPIResp> => {
      console.log("sending: ", script);

      const r = await firstValueFrom(
        ajax.post(`${baseURL}/scripts/${path}`, script).pipe(timeout(2000))
      );

      if (r.status !== 200) {
        return Promise.reject(`Request failed: http status ${r.status}`);
      }

      const resp = r.response as ScriptAPIResp;
      if (resp.error !== undefined) {
        const { message, line, position } = resp.error;
        return Promise.reject(
          `Compile failed: Line ${line}, pos ${position}: ${message}`
        );
      } else {
        return Promise.resolve(resp);
      }
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
