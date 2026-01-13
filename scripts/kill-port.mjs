import { execSync } from "node:child_process"

const port = Number(process.argv[2] || 1501)
if (!Number.isFinite(port)) {
  console.error("Invalid port.")
  process.exit(1)
}

const isWindows = process.platform === "win32"

function exec(cmd) {
  return execSync(cmd, { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] })
}

function killPid(pid) {
  if (!pid) return
  try {
    if (isWindows) {
      exec(`taskkill /PID ${pid} /F`)
    } else {
      process.kill(pid, "SIGKILL")
    }
  } catch {
    // Ignore if already gone or permission denied.
  }
}

function killOnWindows() {
  const output = exec("netstat -ano -p tcp")
  const lines = output.split(/\r?\n/)
  const pids = new Set()
  for (const line of lines) {
    if (!line.includes(`:${port} `)) continue
    if (!line.includes("LISTENING")) continue
    const parts = line.trim().split(/\s+/)
    const pid = Number(parts[parts.length - 1])
    if (Number.isFinite(pid)) pids.add(pid)
  }
  for (const pid of pids) killPid(pid)
  return pids.size
}

function killOnUnix() {
  let pids = []
  try {
    const output = exec(`lsof -ti tcp:${port}`)
    pids = output.split(/\r?\n/).filter(Boolean).map(Number)
  } catch {
    // lsof may be missing; fall back to ss.
    try {
      const output = exec(`ss -lptn "sport = :${port}"`)
      const matches = output.match(/pid=(\d+)/g) || []
      pids = matches.map((m) => Number(m.replace("pid=", "")))
    } catch {
      pids = []
    }
  }
  const unique = [...new Set(pids.filter((pid) => Number.isFinite(pid)))]
  for (const pid of unique) killPid(pid)
  return unique.length
}

const killed = isWindows ? killOnWindows() : killOnUnix()
if (killed > 0) {
  console.log(`Killed ${killed} process(es) on port ${port}.`)
}
