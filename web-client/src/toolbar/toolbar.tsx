import React, { useMemo } from "react";
import { Link, useLocation } from "react-router-dom";
import { LoginButton } from "../auth/login-button";
import { useAuth } from "../hooks/use-auth";
import { ScriptApiBaseUrl } from "../env";

export const Toolbar: React.FC = () => {
  const { isScopeAuthorized } = useAuth();

  const isLocal = useMemo(() => {
    return ScriptApiBaseUrl().includes("localhost")
  }, [])

  return (
    <div className="p-2 bg-dark-gray rounded w-full text-center flex flex-row justify-between">
      <nav className="flex flex-row">
        <NavLink path="/" text="Home" />
        <a
          href="https://siler.github.io/remud"
          className="btn"
          target="_blank"
          rel="noreferrer"
        >
          Docs
        </a>
        {isScopeAuthorized("scripts") && (
          <NavLink path="/scripts" text="Scripts" />
        )}
      </nav>
      <div className="italic">{isLocal ? <span className="success">---- LOCALHOST ----</span> : <>CitySix</>}</div>
      <LoginButton />
    </div>
  );
};

interface LinkProps {
  path: string;
  text: string;
}

const NavLink: React.FC<LinkProps> = ({ path, text }) => {
  const loc = useLocation();
  const isCurrent = path.split("/")[1] === loc.pathname.split("/")[1];

  return (
    <button className={isCurrent ? "btn btn-active" : "btn"}>
      <Link to={path}>{text}</Link>
    </button>
  );
};
