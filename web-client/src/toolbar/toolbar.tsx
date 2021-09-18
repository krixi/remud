import React from "react";
import { Link, useLocation } from "react-router-dom";

export const Toolbar: React.FC = () => {
  return (
    <div className="p-2 bg-dark-gray rounded w-full text-center flex flex-row justify-between">
      <nav className="flex flex-row">
        <NavLink path="/" text="Home" />
        <NavLink path="/scripts" text="Scripts" />
      </nav>
      <span className="italic">CitySix</span>
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
