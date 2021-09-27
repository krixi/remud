import React from "react";
import { Link, useHistory } from "react-router-dom";
import { useAuth } from "../hooks/use-auth";

export const LoginButton: React.FC = () => {
  const { isLoggedIn, user, logout } = useAuth();
  const history = useHistory();

  return isLoggedIn ? (
    <button
      className="btn"
      onClick={() => logout().then(() => history.push("/"))}
    >
      Logout {user?.name}
    </button>
  ) : (
    <button className="btn">
      <Link to="/login">Login</Link>
    </button>
  );
};
