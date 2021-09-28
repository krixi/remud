import { JwtPayload } from "jwt-decode";

export interface LoginReq {
  username: string;
  password: string;
}

export interface RefreshReq {
  refresh_token: string;
}

export interface AuthTokens {
  access_token: string;
  refresh_token: string;
}

export enum AuthActionKind {
  RequestLogin,
  LoginSuccess,
  RefreshSuccess,
  LoginError,
  RefreshError,
  Logout,
}

export interface AuthAction {
  kind: AuthActionKind;
  tokens?: AuthTokens;
}

export interface UserData {
  loading?: boolean;
  tokens?: AuthTokens;
  name?: string;
  expires?: Date;
  scopes?: string[];
}

export interface DecodedToken extends JwtPayload {
  scopes: string[];
}

export interface Auth {
  isLoggedIn: boolean;
  isScopeAuthorized: (scope: string) => boolean;
  user?: UserData;
  login: (req: LoginReq) => Promise<void>;
  logout: () => Promise<void>;
  refresh: () => Promise<AuthTokens>;
}

export interface AuthContext {
  data?: UserData;
  dispatch: (action: AuthAction) => void;
}
