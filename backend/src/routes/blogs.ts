import { Router } from "https://deno.land/x/oak/mod.ts";
import { BlogInput, BlogModel } from "../models/blog.ts";

const router = new Router();

// GET /api/blogs - List all blogs with summaries
router.get("/api/blogs", (ctx) => {
  try {
    const blogs = BlogModel.getAllSummaries();
    ctx.response.body = blogs;
  } catch (error) {
    console.error("Error in GET /api/blogs:", error);
    ctx.response.status = 500;
    ctx.response.body = {
      error: error instanceof Error ? error.message : "Unknown error occurred",
    };
  }
});

// GET /api/blogs/:id - Get a single blog post with full content
router.get("/api/blogs/:id", (ctx) => {
  try {
    const id = ctx.params.id;
    if (!id) {
      ctx.response.status = 400;
      ctx.response.body = { error: "Blog ID is required" };
      return;
    }

    const blog = BlogModel.getById(id);

    if (!blog) {
      ctx.response.status = 404;
      ctx.response.body = { error: "Blog not found" };
      return;
    }

    ctx.response.body = blog;
  } catch (error) {
    console.error(`Error in GET /api/blogs/${ctx.params.id}:`, error);
    ctx.response.status = 500;
    ctx.response.body = {
      error: error instanceof Error ? error.message : "Unknown error occurred",
    };
  }
});

// POST /api/blogs - Create a new blog post
router.post("/api/blogs", async (ctx) => {
  try {
    if (!ctx.request.hasBody) {
      ctx.response.status = 400;
      ctx.response.body = { error: "Request body is missing" };
      return;
    }

    // Parse the request body
    const body = await ctx.request.body;
    let blogInput: BlogInput;

    if (body.type() === "json") {
      blogInput = await body.json();
    } else {
      ctx.response.status = 400;
      ctx.response.body = { error: "Content-Type must be application/json" };
      return;
    }

    // Validate the blog input
    if (!blogInput.title || !blogInput.content) {
      ctx.response.status = 400;
      ctx.response.body = { error: "Title and content are required" };
      return;
    }

    // Create the blog post
    const id = BlogModel.create(blogInput);

    // Return the created blog
    const blog = BlogModel.getById(id);

    ctx.response.status = 201;
    ctx.response.body = blog;
  } catch (error) {
    console.error("Error in POST /api/blogs:", error);
    ctx.response.status = 500;
    ctx.response.body = {
      error: error instanceof Error ? error.message : "Unknown error occurred",
    };
  }
});

// PUT /api/blogs/:id - Update an existing blog post
router.put("/api/blogs/:id", async (ctx) => {
  try {
    const id = ctx.params.id;
    if (!id) {
      ctx.response.status = 400;
      ctx.response.body = { error: "Blog ID is required" };
      return;
    }

    if (!ctx.request.hasBody) {
      ctx.response.status = 400;
      ctx.response.body = { error: "Request body is missing" };
      return;
    }

    // Parse the request body
    const body = await ctx.request.body;
    let blogInput: Partial<BlogInput>;

    if (body.type() === "json") {
      blogInput = await body.json();
    } else {
      ctx.response.status = 400;
      ctx.response.body = { error: "Content-Type must be application/json" };
      return;
    }

    // Update the blog post
    const success = BlogModel.update(id, blogInput);

    if (!success) {
      ctx.response.status = 404;
      ctx.response.body = { error: "Blog not found" };
      return;
    }

    // Return the updated blog
    const blog = BlogModel.getById(id);
    ctx.response.body = blog;
  } catch (error) {
    console.error(`Error in PUT /api/blogs/${ctx.params.id}:`, error);
    ctx.response.status = 500;
    ctx.response.body = {
      error: error instanceof Error ? error.message : "Unknown error occurred",
    };
  }
});

// DELETE /api/blogs/:id - Delete a blog post
router.delete("/api/blogs/:id", async (ctx) => {
  try {
    console.log(`DELETE request for blog ID: ${ctx.params.id}`);
    const id = ctx.params.id;
    if (!id) {
      ctx.response.status = 400;
      ctx.response.body = { error: "Blog ID is required" };
      return;
    }

    // Delete the blog post
    const success = BlogModel.delete(id);
    console.log(`Delete operation success: ${success}`);

    if (!success) {
      ctx.response.status = 404;
      ctx.response.body = { error: "Blog not found" };
      return;
    }

    // Send a successful response with a body instead of 204 No Content
    ctx.response.status = 200;
    ctx.response.body = {
      success: true,
      message: "Blog deleted successfully",
    };
  } catch (error) {
    console.error(`Error in DELETE /api/blogs/${ctx.params.id}:`, error);
    ctx.response.status = 500;
    ctx.response.body = {
      error: error instanceof Error ? error.message : "Unknown error occurred",
    };
  }
});

export const blogsRouter = router;
