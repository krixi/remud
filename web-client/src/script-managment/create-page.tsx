import React from "react";
import { ScriptForm } from "./script-form";

export const CreatePage: React.FC = () => {
  return (
    <div>
      <div className="text-center mb-2">Create a new script</div>
      <ScriptForm isCreate />
    </div>
  );
};
