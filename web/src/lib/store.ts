import { useCallback, useEffect, useState } from "react";

import { ApiError, request, saveTokens, clearTokens, getRefreshToken } from "./api";
import type { AuthResponse, BillingConfig, UserInfo } from "./types";

export interface AuthState {
  user: UserInfo | null;
  unitName: string;
  balance: number;
  loading: boolean;
}

const emptyState: AuthState = {
  user: null,
  unitName: "点",
  balance: 0,
  loading: true
};

export function useAuth() {
  const [state, setState] = useState<AuthState>(emptyState);

  const refresh = useCallback(async () => {
    try {
      const user = await request<UserInfo>("/api/v1/me");
      const wallet = await request<{ balance: number }>("/api/v1/billing/wallet");
      const config = await request<BillingConfig>("/api/v1/billing/config", { auth: false });

      setState({
        user,
        unitName: config.unit_name,
        balance: wallet.balance,
        loading: false
      });
    } catch (err) {
      // 只有真的没登录才清掉凭证。网络抖一下就把人踢出去，体验太差
      const unauthorized = err instanceof ApiError && (err.code === 10002 || err.code === 401);

      if (unauthorized) {
        clearTokens();
        setState({ ...emptyState, loading: false });
      } else {
        setState((prev) => ({ ...prev, loading: false }));
      }
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const login = useCallback(
    async (account: string, password: string) => {
      const result = await request<AuthResponse>("/api/v1/auth/login", {
        method: "POST",
        auth: false,
        body: { account, password }
      });
      saveTokens(result.access_token, result.refresh_token);
      await refresh();
    },
    [refresh]
  );

  const register = useCallback(
    async (phone: string, password: string) => {
      const result = await request<AuthResponse>("/api/v1/auth/register", {
        method: "POST",
        auth: false,
        body: { phone, password }
      });
      saveTokens(result.access_token, result.refresh_token);
      await refresh();
    },
    [refresh]
  );

  const logout = useCallback(async () => {
    try {
      await request("/api/v1/auth/logout", { method: "POST" });
    } finally {
      clearTokens();
      setState({ ...emptyState, loading: false });
    }
  }, []);

  const rotate = useCallback(async () => {
    const token = getRefreshToken();
    if (!token) return;
    const result = await request<AuthResponse>("/api/v1/auth/refresh", {
      method: "POST",
      auth: false,
      body: { refresh_token: token }
    });
    saveTokens(result.access_token, result.refresh_token);
  }, []);

  return { ...state, refresh, login, register, logout, rotate };
}
