import { useCallback, useEffect, useState } from 'react';
import axios, { AxiosError } from 'axios';
import { BACKEND_API_URL } from '../config';
import { LoginResponsePayload, UserProfile } from '../types';

const STORAGE_KEY = 'texler_token';

const getStoredToken = () => {
  if (typeof window === 'undefined') {
    return null;
  }
  return window.localStorage.getItem(STORAGE_KEY);
};

const persistToken = (token: string | null) => {
  if (typeof window === 'undefined') {
    return;
  }
  if (token) {
    window.localStorage.setItem(STORAGE_KEY, token);
  } else {
    window.localStorage.removeItem(STORAGE_KEY);
  }
};

const extractErrorMessage = (error: unknown) => {
  if (axios.isAxiosError(error)) {
    const responseMessage =
      (error.response?.data as { message?: string; error?: string })?.message ||
      (error.response?.data as { error?: string })?.error;
    if (responseMessage) {
      return responseMessage;
    }
  }
  if (error instanceof Error) {
    return error.message;
  }
  return 'Something went wrong';
};

export const useAuth = () => {
  const [token, setToken] = useState<string | null>(getStoredToken);
  const [user, setUser] = useState<UserProfile | null>(null);
  const [loading, setLoading] = useState<boolean>(Boolean(token));
  const [error, setError] = useState<string | null>(null);

  const logout = useCallback(() => {
    persistToken(null);
    setToken(null);
    setUser(null);
    setError(null);
    setLoading(false);
  }, []);

  const fetchProfile = useCallback(
    async (activeToken: string) => {
      try {
        setLoading(true);
        const response = await axios.get(`${BACKEND_API_URL}/api/v1/users/`, {
          headers: {
            Authorization: `Bearer ${activeToken}`,
          },
        });
        const profile = (response.data?.data?.user ?? null) as UserProfile | null;
        if (profile) {
          setUser(profile);
          setError(null);
          return profile;
        }
        throw new Error('Invalid profile payload');
      } catch (err) {
        console.error('Failed to fetch profile', err);
        if ((err as AxiosError).response?.status === 401) {
          logout();
          setError('Session expired. Please log in again.');
        } else {
          setError('Unable to load profile. Please try again.');
        }
        throw err;
      } finally {
        setLoading(false);
      }
    },
    [logout],
  );

  useEffect(() => {
    if (!token) {
      setLoading(false);
      setUser(null);
      return;
    }
    fetchProfile(token).catch(() => {
      /* handled in fetchProfile */
    });
  }, [token, fetchProfile]);

  const login = useCallback(async (email: string, password: string) => {
    setLoading(true);
    setError(null);
    try {
      const response = await axios.post(`${BACKEND_API_URL}/api/v1/auth/login`, {
        email,
        password,
      });
      const data = response.data?.data as LoginResponsePayload | undefined;
      if (!data?.access_token || !data.user) {
        throw new Error('Invalid login response');
      }
      persistToken(data.access_token);
      setToken(data.access_token);
      setUser(data.user);
      return data.user;
    } catch (err) {
      const message = extractErrorMessage(err) || 'Unable to log in';
      setError(message);
      throw err;
    } finally {
      setLoading(false);
    }
  }, []);

  const clearError = useCallback(() => setError(null), []);

  return {
    user,
    token,
    loading,
    error,
    isAuthenticated: Boolean(user && token),
    login,
    logout,
    refetchProfile: () => (token ? fetchProfile(token) : Promise.resolve(null)),
    clearError,
  };
};
