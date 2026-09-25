-- :DequePreview: `deque preview` in a terminal split, fed the buffer (saved
-- or not) and the cursor's line through a file, so it shows the slide the
-- cursor is in as you type, played in as the cursor comes to it.
-- :DequePlay plays it again. vim.g.deque_preview_still = true keeps it still.
local M = {}

-- The previews open, by the talk's buffer: { win, file, timer, group, plays }.
local previews = {}

-- The cursor's line and the replays asked for, then the buffer, written
-- whole and moved into place so deque never reads half of it.
local function write(src)
  local p = previews[src]
  if not p then return end
  local row = 0
  local win = vim.fn.bufwinid(src)
  if win ~= -1 then row = vim.api.nvim_win_get_cursor(win)[1] - 1 end
  local lines = vim.api.nvim_buf_get_lines(src, 0, -1, false)
  table.insert(lines, 1, row .. " " .. p.plays)
  vim.fn.writefile(lines, p.file .. ".new")
  vim.uv.fs_rename(p.file .. ".new", p.file)
end

function M.close(src)
  local p = previews[src]
  if not p then return end
  previews[src] = nil
  p.timer:stop()
  p.timer:close()
  pcall(vim.api.nvim_del_augroup_by_id, p.group)
  if vim.api.nvim_win_is_valid(p.win) then
    local buf = vim.api.nvim_win_get_buf(p.win)
    pcall(vim.api.nvim_win_close, p.win, true)
    pcall(vim.api.nvim_buf_delete, buf, { force = true })
  end
  os.remove(p.file)
end

function M.open(src)
  local path = vim.api.nvim_buf_get_name(src)
  if path == "" then
    vim.notify("deque: save the talk first; pictures are found beside it", vim.log.levels.WARN)
    return
  end
  if vim.fn.executable("deque") == 0 then
    vim.notify("deque: not on PATH; see https://github.com/booka66/deque#install", vim.log.levels.ERROR)
    return
  end
  local p = { file = vim.fn.tempname() .. ".deque-preview", timer = vim.uv.new_timer(), plays = 0 }
  previews[src] = p
  write(src)

  local from = vim.api.nvim_get_current_win()
  vim.cmd("botright vsplit")
  p.win = vim.api.nvim_get_current_win()
  vim.api.nvim_win_set_width(p.win, math.max(40, math.floor(vim.o.columns * 0.45)))
  vim.api.nvim_win_set_buf(p.win, vim.api.nvim_create_buf(false, true))
  for k, v in pairs({ number = false, relativenumber = false, signcolumn = "no", cursorline = false, statuscolumn = "", foldcolumn = "0" }) do
    vim.wo[p.win][k] = v
  end
  local cmd = { "deque", "preview", path, "--from", p.file }
  if vim.g.deque_preview_still then table.insert(cmd, "--still") end
  vim.fn.jobstart(cmd, {
    term = true,
    on_exit = function() vim.schedule(function() M.close(src) end) end,
  })
  vim.api.nvim_set_current_win(from)

  -- Written again a moment after each change or move, not on every key.
  p.group = vim.api.nvim_create_augroup("deque_preview_" .. src, { clear = true })
  vim.api.nvim_create_autocmd({ "TextChanged", "TextChangedI", "CursorMoved", "CursorMovedI" }, {
    group = p.group,
    buffer = src,
    callback = function()
      p.timer:stop()
      p.timer:start(50, 0, vim.schedule_wrap(function() write(src) end))
    end,
  })
  vim.api.nvim_create_autocmd({ "BufWipeout", "BufUnload" }, {
    group = p.group,
    buffer = src,
    callback = function() M.close(src) end,
  })
  vim.api.nvim_create_autocmd("WinClosed", {
    group = p.group,
    pattern = tostring(p.win),
    callback = function() vim.schedule(function() M.close(src) end) end,
  })
end

-- The slide under the cursor played in again.
function M.play()
  local src = vim.api.nvim_get_current_buf()
  if not previews[src] then M.open(src) return end
  previews[src].plays = previews[src].plays + 1
  write(src)
end

function M.toggle()
  local src = vim.api.nvim_get_current_buf()
  if previews[src] then M.close(src) else M.open(src) end
end

return M
