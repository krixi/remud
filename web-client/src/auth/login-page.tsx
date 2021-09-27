import React from "react";
import { LoginForm } from "./login-form";

export const LoginPage: React.FC = () => {
  return (
    <div className="max-w-xl container mx-auto text-center mt-10">
      <LoginForm />
    </div>
  );
};
