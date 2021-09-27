import { useCallback, useMemo } from "react";
import { useHistory } from "react-router-dom";
import { useAuthContext } from "./use-auth-context";
import { Auth, AuthActionKind } from "../models/auth-api";

export const useAuth = (): Auth => {
  const { data, dispatch } = useAuthContext();
  const history = useHistory();

  const logout = useCallback(() => {
    dispatch({ kind: AuthActionKind.Logout });
    history.push("/");
  }, [dispatch, history]);

  const isLoggedIn = useMemo(() => data?.tokens !== undefined, [data]);

  return {
    isLoggedIn,
    user: data,
    logout,
  };
};
