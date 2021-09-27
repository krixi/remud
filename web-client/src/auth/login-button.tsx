import React from "react";
import { Link } from "react-router-dom";
import { useAuth } from "../hooks/use-auth";

export const LoginButton: React.FC = () => {
  const { isLoggedIn, logout } = useAuth();

  return isLoggedIn ? (
    <button className="btn" onClick={logout}>
      Logout
    </button>
  ) : (
    <button className="btn">
      <Link to="/login">Login</Link>
    </button>
  );
};
