import React from "react";
import { useGetScript } from "../hooks/use-get-script";
import { ScriptAPIBaseURL } from "../env";
import { ErrorDisplay } from "./error-display";
import { ScriptForm } from "./script-form";

export interface PublicProps {
  name: string;
}

export const EditPage: React.FC<PublicProps> = ({ name }) => {
  const { script, loading, err } = useGetScript(ScriptAPIBaseURL, name);

  if (err) {
    return <ErrorDisplay err={err} />;
  }

  if (!script && !loading) {
    return (
      <div>
        Couldn't find script named <pre>{name}</pre>
      </div>
    );
  }

  return <ScriptForm isCreate={false} script={script} />;
};
