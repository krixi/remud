import React from "react";
import { Toolbar } from "./toolbar/toolbar";
import { Route, Switch } from "react-router-dom";
import { ScriptManagementPage } from "./script-managment/script-management";

export const App: React.FC = () => {
  return (
    <div className="bg-soft-gray w-full h-screen text-white">
      <Toolbar />
      <Switch>
        <Route path="/scripts/:name">
          <ScriptManagementPage />
        </Route>
        <Route path="/scripts">
          <ScriptManagementPage />
        </Route>
        <Route path="/">
          <div className="text-center m-2">
            <p>Welcome to the web console. </p>
          </div>
        </Route>
      </Switch>
    </div>
  );
};
