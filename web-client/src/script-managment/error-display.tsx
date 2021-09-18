import React from "react";

export interface PublicProps {
  err: Error;
}

export const ErrorDisplay: React.FC<PublicProps> = ({ err }) => {
  return (
    <div>
      There was an error processing the request, please try again later.
      <pre>{err.message}</pre>
    </div>
  );
};
