import { loginRedirect, logout } from '../utils/api';

interface LoginButtonProps {
  isAuthenticated: boolean;
  userName?: string;
  onLogoutComplete?: () => void;
}

export function LoginButton({ isAuthenticated, userName, onLogoutComplete }: LoginButtonProps) {
  const handleLogout = async () => {
    try {
      await logout();
      onLogoutComplete?.();
    } catch (err) {
      console.error('Logout failed:', err);
    }
  };

  if (isAuthenticated) {
    return (
      <div className="login-container">
        {userName && <span className="user-name">{userName}</span>}
        <button
          onClick={handleLogout}
          className="btn btn-secondary"
          aria-label="Log out"
        >
          Logout
        </button>
      </div>
    );
  }

  return (
    <button
      onClick={loginRedirect}
      className="btn btn-primary"
      aria-label="Log in"
    >
      Login
    </button>
  );
}
