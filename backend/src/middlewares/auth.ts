// src/middlewares/auth.ts
import { Context, Next } from "https://deno.land/x/oak/mod.ts";

export async function authMiddleware(ctx: Context, next: Next) {
    // This is a placeholder for future authentication
    // For now, we'll just pass through all requests
    await next();
}