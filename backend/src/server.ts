import { Application } from "https://deno.land/x/oak/mod.ts";
import { blogsRouter } from "./routes/blogs.ts";
import { tagsRouter } from "./routes/tags.ts";
import { authMiddleware } from "./middlewares/auth.ts";
import { initDb } from "./utils/db.ts";

initDb(); // Initialize the database

const app = new Application();

// Logger middleware
app.use(async (ctx, next) => {
  await next();
  const rt = ctx.response.headers.get("X-Response-Time");
  console.log(`${ctx.request.method} ${ctx.request.url} - ${rt}`);
});

// Response time middleware
app.use(async (ctx, next) => {
  const start = Date.now();
  await next();
  const ms = Date.now() - start;
  ctx.response.headers.set("X-Response-Time", `${ms}ms`);
});

// CORS middleware
app.use(async (ctx, next) => {
  ctx.response.headers.set("Access-Control-Allow-Origin", "*");
  ctx.response.headers.set(
    "Access-Control-Allow-Methods",
    "GET, POST, PUT, DELETE, OPTIONS",
  );
  ctx.response.headers.set(
    "Access-Control-Allow-Headers",
    "Content-Type, Authorization, Accept",
  );
  ctx.response.headers.set("Access-Control-Max-Age", "3600");

  // Handle preflight requests
  if (ctx.request.method === "OPTIONS") {
    ctx.response.status = 204; // No content for OPTIONS
    return;
  }

  await next();
});

// Auth middleware
app.use(authMiddleware);

// Routes
app.use(blogsRouter.routes());
app.use(blogsRouter.allowedMethods());
app.use(tagsRouter.routes());
app.use(tagsRouter.allowedMethods());

// Error handling
app.use(async (ctx, next) => {
  try {
    await next();
  } catch (err) {
    console.error("Unhandled error:", err);
    ctx.response.status = 500;
    ctx.response.body = { error: "Internal server error" };
  }
});

// Start the server
const port = 8000;
console.log(`Server running on http://localhost:${port}`);

await app.listen({ port });
