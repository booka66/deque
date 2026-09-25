-- deque talk files: the filetype, markdown highlighting, and :DequePreview,
-- a split beside the talk that shows the slide the cursor is in.
vim.filetype.add({ extension = { deque = "deque" } })
vim.treesitter.language.register("markdown", "deque")

vim.api.nvim_create_autocmd("FileType", {
  pattern = "deque",
  callback = function(ev)
    -- // notes and > steps carry on to the next line; # headlines don't.
    vim.bo[ev.buf].comments = "://,n:>"
    vim.bo[ev.buf].commentstring = "// %s"
    -- With coc, only deque's own suggestions: its word sources would offer
    -- the talk's words (SCRAMBLE from a headline) beside the real options.
    vim.b[ev.buf].coc_disabled_sources = { "around", "buffer" }
    vim.keymap.set("n", "<localleader>p", function() require("deque").toggle() end,
      { buffer = ev.buf, desc = "deque: preview the slide under the cursor" })
    vim.keymap.set("n", "<localleader>r", function() require("deque").play() end,
      { buffer = ev.buf, desc = "deque: play the slide under the cursor again" })
  end,
})

vim.api.nvim_create_user_command("DequePlay", function() require("deque").play() end,
  { desc = "deque: play the slide under the cursor again, in the preview" })
vim.api.nvim_create_user_command("DequePreview", function() require("deque").toggle() end,
  { desc = "deque: preview the slide under the cursor, beside the talk" })
