import React, { FormEvent, useCallback, useMemo, useState } from "react";
import { useHistory } from "react-router-dom";
import { useAuth } from "../hooks/use-auth";

export const LoginForm: React.FC = () => {
  const history = useHistory();
  const { login, user } = useAuth();
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
        .catch((reason) => {
          setErr("login failed");
          console.log(reason);
        });
    },
    [history, login]
  );

  const loading = useMemo(() => user?.loading || false, [user]);

  return (
    <form
      className="w-full"
      onSubmit={(e) => onSubmit(e, username.trim(), password)}
    >
      <div className="italic font-mono">ucs://uplink.six.city admin node</div>
      <div className="italic font-mono mb-10">
        {err ? (
          <span className="text-red-600">{err}</span>
        ) : loading ? (
          <>processing...</>
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
