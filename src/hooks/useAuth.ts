import { useCallback, useEffect, useState } from 'react';
import { getAuthStatus } from '../utils/api';
import type { User } from '../types';

export function useAuth() {
  const [user, setUser] = useState<User | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const refreshAuth = useCallback(async () => {
    setLoading(true);
    try {
      const authStatus = await getAuthStatus();
      setUser(authStatus);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to fetch auth status');
      setUser(null);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refreshAuth();
  }, [refreshAuth]);

  return {
    user,
    loading,
    error,
    isAuthenticated: user?.authenticated ?? false,
    refreshAuth,
  };
}
