import React from "react";
import { useParams } from "react-router-dom";
import { EditPage } from "./edit-page";
import { ListScripts } from "./list-scripts";
import { CreatePage } from "./create-page";

interface ScriptsParam {
  name: string;
}

export const ScriptManagementPage: React.FC = () => {
  const { name } = useParams<ScriptsParam>();
  // if the name is 'new', we cam here from the 'Create Script' button
  const shouldCreate = name?.toLowerCase() === "new";

  return (
    <div className="m-2 p-2 h-96">
      {shouldCreate ? (
        <CreatePage />
      ) : name ? (
        <EditPage name={name} />
      ) : (
        <ListScripts />
      )}
    </div>
  );
};
