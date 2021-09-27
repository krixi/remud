import React, { FormEvent, useCallback, useState } from "react";
import { useLoginApi } from "../hooks/use-login-api";
import { ScriptApiBaseUrl } from "../env";
import { useHistory } from "react-router-dom";

export const LoginForm: React.FC = () => {
  const history = useHistory();
  const login = useLoginApi(ScriptApiBaseUrl());
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [err, setErr] = useState<string | undefined>();

  const onSubmit = useCallback(
    (e: FormEvent, username?: string, password?: string) => {
      e.preventDefault();
      setErr(undefined);
      if (!username || !password) {
        setErr("name and password are required");
        return;
      }

      login({ username, password })
        .then(() => history.push("/")) // redirect to home on login
        .catch((reason) => console.log("couldn't log in", reason));
    },
    [history, login]
  );

  return (
    <form
      className="w-full"
      onSubmit={(e) => onSubmit(e, username.trim(), password.trim())}
    >
      <div className="italic font-mono">ucs://uplink.six.city admin node</div>
      <div className="italic font-mono mb-10">
        {err ? (
          <span className="text-red-600">{err}</span>
        ) : (
          <>credentials required</>
        )}
      </div>
      <div className="flex flex-row justify-between mb-10">
        <label htmlFor="name" className="w-1/3 p-1">
          Name
        </label>
        <input
          id="name"
          className="w-2/3 text-black rounded-sm p-1"
          type="text"
          value={username}
          onChange={(e) => setUsername(e.target.value)}
        />
      </div>
      <div className="flex flex-row justify-between mb-10">
        <label htmlFor="pass" className="w-1/3 p-1">
          Password
        </label>
        <input
          id="pass"
          className="w-2/3 text-black rounded-sm p-1"
          type="password"
          value={password}
          onChange={(e) => setPassword(e.target.value)}
        />
      </div>
      <input
        className="btn w-2/3 cursor-pointer bg-soft-gray"
        type="submit"
        value="Request Access"
      />
    </form>
  );
};
