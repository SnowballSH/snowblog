import { Router } from "https://deno.land/x/oak/mod.ts";
import { TagModel } from "../models/tag.ts";

const router = new Router();

// GET /api/tags - List all tags with counts
router.get("/api/tags", (ctx) => {
  try {
    const tags = TagModel.getAll();
    ctx.response.body = tags;
  } catch (error) {
    console.error("Error in GET /api/tags:", error);
    ctx.response.status = 500;
    ctx.response.body = {
      error: error instanceof Error ? error.message : "Unknown error occurred",
    };
  }
});

// GET /api/tags/:id - Get a single tag
router.get("/api/tags/:id", (ctx) => {
  try {
    const id = ctx.params.id;
    if (!id) {
      ctx.response.status = 400;
      ctx.response.body = { error: "Tag ID is required" };
      return;
    }

    const tag = TagModel.getById(id);

    if (!tag) {
      ctx.response.status = 404;
      ctx.response.body = { error: "Tag not found" };
      return;
    }

    ctx.response.body = tag;
  } catch (error) {
    console.error(`Error in GET /api/tags/${ctx.params.id}:`, error);
    ctx.response.status = 500;
    ctx.response.body = {
      error: error instanceof Error ? error.message : "Unknown error occurred",
    };
  }
});

// POST /api/tags - Create a new tag
router.post("/api/tags", async (ctx) => {
  try {
    if (!ctx.request.hasBody) {
      ctx.response.status = 400;
      ctx.response.body = { error: "Request body is missing" };
      return;
    }

    // Parse the request body
    const body = await ctx.request.body;
    let tagName: string;

    if (body.type() === "json") {
      const data = await body.json();
      tagName = data.name;
    } else {
      ctx.response.status = 400;
      ctx.response.body = { error: "Content-Type must be application/json" };
      return;
    }

    // Validate tag name
    if (!tagName || !tagName.trim()) {
      ctx.response.status = 400;
      ctx.response.body = { error: "Tag name is required" };
      return;
    }

    // Create the tag
    const id = TagModel.create(tagName);

    // Return the created tag
    const tag = TagModel.getById(id);

    ctx.response.status = 201;
    ctx.response.body = tag;
  } catch (error) {
    console.error("Error in POST /api/tags:", error);
    ctx.response.status = 500;
    ctx.response.body = {
      error: error instanceof Error ? error.message : "Unknown error occurred",
    };
  }
});

// PUT /api/tags/:id - Update a tag
router.put("/api/tags/:id", async (ctx) => {
  try {
    const id = ctx.params.id;
    if (!id) {
      ctx.response.status = 400;
      ctx.response.body = { error: "Tag ID is required" };
      return;
    }

    if (!ctx.request.hasBody) {
      ctx.response.status = 400;
      ctx.response.body = { error: "Request body is missing" };
      return;
    }

    // Parse the request body
    const body = await ctx.request.body;
    let tagName: string;

    if (body.type() === "json") {
      const data = await body.json();
      tagName = data.name;
    } else {
      ctx.response.status = 400;
      ctx.response.body = { error: "Content-Type must be application/json" };
      return;
    }

    // Validate tag name
    if (!tagName || !tagName.trim()) {
      ctx.response.status = 400;
      ctx.response.body = { error: "Tag name is required" };
      return;
    }

    // Update the tag
    const success = TagModel.update(id, tagName);

    if (!success) {
      ctx.response.status = 404;
      ctx.response.body = { error: "Tag not found" };
      return;
    }

    // Return the updated tag
    const tag = TagModel.getById(id);
    ctx.response.body = tag;
  } catch (error) {
    console.error(`Error in PUT /api/tags/${ctx.params.id}:`, error);
    ctx.response.status = 500;
    ctx.response.body = {
      error: error instanceof Error ? error.message : "Unknown error occurred",
    };
  }
});

// DELETE /api/tags/:id - Delete a tag
router.delete("/api/tags/:id", (ctx) => {
  try {
    const id = ctx.params.id;
    if (!id) {
      ctx.response.status = 400;
      ctx.response.body = { error: "Tag ID is required" };
      return;
    }

    const success = TagModel.delete(id);

    if (!success) {
      ctx.response.status = 404;
      ctx.response.body = { error: "Tag not found" };
      return;
    }

    ctx.response.status = 204; // No content
  } catch (error) {
    console.error(`Error in DELETE /api/tags/${ctx.params.id}:`, error);
    ctx.response.status = 500;
    ctx.response.body = {
      error: error instanceof Error ? error.message : "Unknown error occurred",
    };
  }
});

// GET /api/tags/:name/blogs - Get blogs by tag name
router.get("/api/tags/:name/blogs", (ctx) => {
  try {
    const tagName = ctx.params.name;
    if (!tagName) {
      ctx.response.status = 400;
      ctx.response.body = { error: "Tag name is required" };
      return;
    }

    const blogIds = TagModel.getBlogsByTagName(tagName);
    ctx.response.body = blogIds;
  } catch (error) {
    console.error(`Error in GET /api/tags/${ctx.params.name}/blogs:`, error);
    ctx.response.status = 500;
    ctx.response.body = {
      error: error instanceof Error ? error.message : "Unknown error occurred",
    };
  }
});

export const tagsRouter = router;
