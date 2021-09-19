import React from "react";
import { CompileError } from "../models/scripts-api";

export interface CompileStatusCheckboxProps {
  error?: CompileError;
}

// icons courtesy of https://heroicons.com/ (MIT licensed)
export const CompileStatusCheckbox: React.FC<CompileStatusCheckboxProps> = ({
  error,
}) => {
  if (!error) {
    return (
      <div>
        <span title="Compiled successfully" className="success">
          <svg
            xmlns="http://www.w3.org/2000/svg"
            className="h-6 w-6"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M5 13l4 4L19 7"
            />
          </svg>
        </span>
      </div>
    );
  } else {
    return (
      <div>
        <span title={error.message} className="text-red-600">
          <svg
            xmlns="http://www.w3.org/2000/svg"
            className="h-6 w-6"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M6 18L18 6M6 6l12 12"
            />
          </svg>
        </span>
      </div>
    );
  }
};
