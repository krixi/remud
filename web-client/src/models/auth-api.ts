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

export interface UserData {
  loading?: boolean;
  tokens?: AuthTokens;
  name?: string;
  expires?: Date;
}

export interface Auth {
  isLoggedIn: boolean;
  user?: UserData;
  login: (req: LoginReq) => Promise<void>;
  logout: () => Promise<void>;
}

export interface AuthContext {
  data?: UserData;
  dispatch: (action: AuthAction) => void;
}
