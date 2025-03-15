/**
 * Andy Coin Discord Bot - Cloudflare Workers Proxy
 * 
 * This worker acts as a proxy and health check for the Andy Coin Discord bot.
 * It can be used to:
 * 1. Keep the bot alive by periodically pinging it
 * 2. Store backup data in Cloudflare KV and D1
 * 3. Provide a simple status page
 */

// Handle incoming requests
export default {
  async fetch(request, env, ctx) {
    const url = new URL(request.url);
    
    // Simple status page
    if (url.pathname === "/" || url.pathname === "/status") {
      return handleStatusPage(request, env);
    }
    
    // API endpoints
    if (url.pathname.startsWith("/api/")) {
      return handleApiRequest(request, env);
    }
    
    // Default response for unhandled routes
    return new Response("Not Found", { status: 404 });
  },
  
  // Scheduled task to keep the bot alive and perform backups
  async scheduled(event, env, ctx) {
    console.log("Running scheduled task:", event.scheduledTime);
    
    try {
      // Ping the bot to keep it alive
      const pingResponse = await fetch(`${env.DISCORD_BOT_URL}/health`, {
        method: "GET",
        headers: {
          "User-Agent": "Cloudflare-Worker-Andy-Coin-Monitor"
        }
      });
      
      const status = pingResponse.ok ? "healthy" : "unhealthy";
      const timestamp = new Date().toISOString();
      
      // Store health check result in KV
      await env.ANDY_COIN_KV.put("last_health_check", JSON.stringify({
        status,
        timestamp,
        statusCode: pingResponse.status
      }));
      
      // If the bot is healthy, request a data backup
      if (pingResponse.ok) {
        await requestDataBackup(env);
      }
      
      console.log(`Health check completed: ${status}`);
    } catch (error) {
      console.error("Error in scheduled task:", error);
      
      // Store error in KV
      await env.ANDY_COIN_KV.put("last_health_check", JSON.stringify({
        status: "error",
        timestamp: new Date().toISOString(),
        error: error.message
      }));
    }
  }
};

// Handle status page requests
async function handleStatusPage(request, env) {
  try {
    // Get the last health check result
    const lastHealthCheck = await env.ANDY_COIN_KV.get("last_health_check", { type: "json" });
    
    // Get some basic stats from D1
    const { results: stats } = await env.DB.prepare(
      "SELECT COUNT(*) as total_guilds FROM guild_stats"
    ).all();
    
    const html = `
      <!DOCTYPE html>
      <html lang="en">
      <head>
        <meta charset="UTF-8">
        <meta name="viewport" content="width=device-width, initial-scale=1.0">
        <title>Andy Coin Status</title>
        <style>
          body {
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
            line-height: 1.6;
            color: #333;
            max-width: 800px;
            margin: 0 auto;
            padding: 20px;
          }
          h1 {
            color: #2563eb;
          }
          .status {
            padding: 15px;
            border-radius: 5px;
            margin-bottom: 20px;
          }
          .healthy {
            background-color: #d1fae5;
            border: 1px solid #10b981;
          }
          .unhealthy {
            background-color: #fee2e2;
            border: 1px solid #ef4444;
          }
          .unknown {
            background-color: #f3f4f6;
            border: 1px solid #9ca3af;
          }
          .stats {
            background-color: #f3f4f6;
            border: 1px solid #d1d5db;
            padding: 15px;
            border-radius: 5px;
          }
        </style>
      </head>
      <body>
        <h1>Andy Coin Status</h1>
        
        <div class="status ${lastHealthCheck?.status || 'unknown'}">
          <h2>Bot Status: ${lastHealthCheck?.status || 'Unknown'}</h2>
          <p>Last checked: ${lastHealthCheck?.timestamp || 'Never'}</p>
          ${lastHealthCheck?.error ? `<p>Error: ${lastHealthCheck.error}</p>` : ''}
        </div>
        
        <div class="stats">
          <h2>Statistics</h2>
          <p>Total Guilds: ${stats?.[0]?.total_guilds || 'Unknown'}</p>
        </div>
      </body>
      </html>
    `;
    
    return new Response(html, {
      headers: {
        "Content-Type": "text/html;charset=UTF-8"
      }
    });
  } catch (error) {
    console.error("Error generating status page:", error);
    return new Response("Error generating status page", { status: 500 });
  }
}

// Handle API requests
async function handleApiRequest(request, env) {
  const url = new URL(request.url);
  
  // Handle backup data submission
  if (url.pathname === "/api/backup" && request.method === "POST") {
    return handleBackupData(request, env);
  }
  
  // Handle health check
  if (url.pathname === "/api/health" && request.method === "GET") {
    return new Response(JSON.stringify({ status: "ok" }), {
      headers: { "Content-Type": "application/json" }
    });
  }
  
  return new Response("Not Found", { status: 404 });
}

// Handle backup data submission
async function handleBackupData(request, env) {
  try {
    // Verify authorization
    const authHeader = request.headers.get("Authorization");
    if (!authHeader || !authHeader.startsWith("Bearer ")) {
      return new Response("Unauthorized", { status: 401 });
    }
    
    // In a real implementation, you would verify the token
    // const token = authHeader.split(" ")[1];
    // if (token !== env.API_TOKEN) {
    //   return new Response("Unauthorized", { status: 401 });
    // }
    
    // Parse the backup data
    const data = await request.json();
    
    // Store the full backup in KV
    const timestamp = new Date().toISOString();
    await env.ANDY_COIN_KV.put(`backup_${timestamp}`, JSON.stringify(data));
    
    // Store guild stats in D1
    if (data.balances && Array.isArray(data.balances)) {
      // Group balances by guild
      const guildBalances = {};
      for (const balance of data.balances) {
        if (!guildBalances[balance.guild_id]) {
          guildBalances[balance.guild_id] = {
            guild_id: balance.guild_id,
            user_count: 0,
            total_coins: 0
          };
        }
        
        guildBalances[balance.guild_id].user_count++;
        guildBalances[balance.guild_id].total_coins += balance.balance;
      }
      
      // Insert guild stats into D1
      const stmt = env.DB.prepare(`
        INSERT INTO guild_stats (guild_id, user_count, total_coins, timestamp)
        VALUES (?, ?, ?, ?)
      `);
      
      for (const guildId in guildBalances) {
        const stats = guildBalances[guildId];
        await stmt.bind(
          stats.guild_id.toString(),
          stats.user_count,
          stats.total_coins,
          timestamp
        ).run();
      }
    }
    
    return new Response(JSON.stringify({ success: true, timestamp }), {
      headers: { "Content-Type": "application/json" }
    });
  } catch (error) {
    console.error("Error handling backup data:", error);
    return new Response(JSON.stringify({ error: error.message }), {
      status: 500,
      headers: { "Content-Type": "application/json" }
    });
  }
}

// Request a data backup from the bot
async function requestDataBackup(env) {
  try {
    const response = await fetch(`${env.DISCORD_BOT_URL}/backup`, {
      method: "GET",
      headers: {
        "User-Agent": "Cloudflare-Worker-Andy-Coin-Monitor",
        "Authorization": `Bearer ${env.API_TOKEN}`
      }
    });
    
    if (!response.ok) {
      throw new Error(`Failed to request backup: ${response.status}`);
    }
    
    console.log("Backup requested successfully");
  } catch (error) {
    console.error("Error requesting backup:", error);
  }
}
