import React, { useState } from 'react';

interface LoginPanelProps {
  onLogin: (email: string, password: string) => Promise<unknown>;
  loading: boolean;
  error?: string | null;
}

export const LoginPanel: React.FC<LoginPanelProps> = ({ onLogin, loading, error }) => {
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [localError, setLocalError] = useState<string | null>(null);

  const handleSubmit = async (event: React.FormEvent) => {
    event.preventDefault();
    if (!email.trim() || !password) {
      setLocalError('Email and password are required');
      return;
    }
    setLocalError(null);
    try {
      await onLogin(email.trim(), password);
    } catch {
      // Error is surfaced via props.error from the auth hook
    }
  };

  const helperText = localError || error;

  return (
    <div className="login-screen">
      <div className="login-card">
        <div className="login-header">
          <h1>Texler</h1>
          <p>Sign in to access your workspaces and projects</p>
        </div>
        <form className="login-form" onSubmit={handleSubmit}>
          <label className="login-label" htmlFor="email">
            Email
          </label>
          <input
            id="email"
            type="email"
            autoComplete="email"
            className="login-input"
            placeholder="you@example.com"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            disabled={loading}
          />
          <label className="login-label" htmlFor="password">
            Password
          </label>
          <input
            id="password"
            type="password"
            autoComplete="current-password"
            className="login-input"
            placeholder="••••••••"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            disabled={loading}
          />
          {helperText && <p className="login-error">{helperText}</p>}
          <button type="submit" className="login-button" disabled={loading}>
            {loading ? 'Signing in…' : 'Sign in'}
          </button>
        </form>
        <div className="login-footer">
          <p>Need an account? Use the CLI or API to register a user.</p>
        </div>
      </div>
    </div>
  );
};
