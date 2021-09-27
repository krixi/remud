import { useEffect, useState } from "react";
import { ScriptInfo } from "../models/scripts-api";
import { useScriptsApi } from "./use-scripts-api";

const sortScripts = (a: ScriptInfo, b: ScriptInfo): number => {
  if (a.name < b.name) {
    return -1;
  }
  if (a.name > b.name) {
    return 1;
  }
  return 0;
};

export const useListScripts = (baseURL: string) => {
  const { list } = useScriptsApi(baseURL);
  const [scripts, setScripts] = useState<ScriptInfo[]>();
  const [loading, setLoading] = useState(true);
  const [err, setErr] = useState<Error>();

  useEffect(() => {
    setLoading(true);
    setErr(undefined);
    list()
      .then((r) => setScripts(r.sort(sortScripts)))
      .catch((err) => setErr(err))
      .finally(() => setLoading(false));
  }, [list]);

  return { scripts, loading, err };
};
