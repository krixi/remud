import React from "react";
import { useListScripts } from "../hooks/use-list-scripts";
import { ScriptApiBaseUrl } from "../env";
import { Link } from "react-router-dom";
import { ErrorDisplay } from "./error-display";
import { CompileError } from "../models/scripts-api";

export const ListScripts: React.FC = () => {
  const { scripts, loading, err } = useListScripts(ScriptApiBaseUrl());
  return (
    <div>
      <div className="flex flex-row">
        <button className="btn">
          <Link to="/scripts/new">Create Script</Link>
        </button>
      </div>
      {scripts ? (
        <div className="mt-2 flex flex-col">
          <div className="flex flex-row justify-between border-b border-gray-500 mb-1">
            <span className="w-full font-bold">Script name</span>
            <span className="w-full font-bold">Trigger</span>
            <span className="w-full font-bold"># of lines</span>
            <span className="w-full font-bold">Status</span>
            <span className="w-full font-bold">&nbsp;</span>
          </div>
          {scripts.map((s) => (
            <div key={s.name} className="flex flex-row justify-between">
              <span className="w-full">
                <pre className="inline-flex">{s.name}</pre>
              </span>
              <span className="w-full">
                {s.trigger && <pre className="inline-flex">{s.trigger}</pre>}
              </span>
              <span className="w-full">{s.lines && <>{s.lines} lines</>}</span>
              <span className="w-full"><CompileStatusCheckbox error={s.error}></CompileStatusCheckbox></span>
              <span className="w-full">
                <button className="btn">
                  <Link to={`/scripts/${s.name}`}>Edit</Link>
                </button>
              </span>
            </div>
          ))}
        </div>
      ) : err ? (
        <ErrorDisplay err={err} />
      ) : (
        !loading && <div>No scripts found, perhaps you want to make one?</div>
      )}
    </div>
  );
};


interface CompileStatusCheckboxProps {
  error?: CompileError
};

const CompileStatusCheckbox: React.FC<CompileStatusCheckboxProps> = ({
  error
}) => {
  if (!error) {
    return (<div><span title="Compiled successfully">✔️</span></div>)
  } else {
    return (<div><span title={error.message}>❌</span></div>)
  }
}