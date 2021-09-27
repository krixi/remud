export interface LoginReq {
  username: string;
  password: string;
}

export interface AuthTokens {
  access_token: string;
  refresh_token: string;
}

export enum AuthActionKind {
  RequestLogin,
  LoginSuccess,
  LoginError,
  Logout,
}

export interface AuthAction {
  kind: AuthActionKind;
  tokens?: AuthTokens;
}

export interface AuthData {
  loading?: boolean;
  tokens?: AuthTokens;
}

export interface Auth {
  isLoggedIn: boolean;
  user?: AuthData;
  logout: () => Promise<void>;
}

export interface AuthContext {
  data?: AuthData;
  dispatch: (action: AuthAction) => void;
}
