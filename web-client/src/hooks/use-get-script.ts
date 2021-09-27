import { useEffect, useState } from "react";
import { Script } from "../models/scripts-api";
import { useScriptsApi } from "./use-scripts-api";

export const useGetScript = (baseURL: string, name: string) => {
  const { get } = useScriptsApi(baseURL);
  const [script, setScript] = useState<Script>();
  const [loading, setLoading] = useState(true);
  const [err, setErr] = useState<Error>();

  useEffect(() => {
    setErr(undefined);
    setLoading(true);
    get(name)
      .then((script) => setScript(script))
      .catch((err) => setErr(err))
      .finally(() => setLoading(false));
  }, [baseURL, name, get]);

  return { script, loading, err };
};
