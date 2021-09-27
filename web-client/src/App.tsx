import React from "react";
import { Route, Switch } from "react-router-dom";
import { AuthProvider } from "./auth/auth-context";
import { LoginPage } from "./auth/login-page";
import { ScriptManagementPage } from "./script-managment/script-management";
import { Toolbar } from "./toolbar/toolbar";

export const App: React.FC = () => {
  return (
    <AuthProvider>
      <div className="bg-soft-gray w-full h-screen text-white">
        <Toolbar />
        <Switch>
          <Route path="/scripts/:name">
            <ScriptManagementPage />
          </Route>
          <Route path="/scripts">
            <ScriptManagementPage />
          </Route>
          <Route path="/login">
            <LoginPage />
          </Route>
          <Route path="/">
            <div className="text-center m-2">
              <p>Welcome to the web console. </p>
            </div>
          </Route>
        </Switch>
      </div>
    </AuthProvider>
  );
};
