using Spectre.Console;
using System.Text.RegularExpressions;

// Server Deployment Wizard - A comprehensive test of Spectre.Console interactive features
// This CLI exercises every problematic interaction pattern for AI agents:
// - Arrow-key navigation
// - Spacebar toggles
// - Hidden input (secret prompts)
// - Validation loops
// - Type coercion
// - Hierarchical navigation
// - Search/filter
// - Non-standard confirm characters
// - Progress bars
// - Live displays

var config = new DeploymentConfig();

// Step 1: Welcome + Confirmation
AnsiConsole.Clear();
AnsiConsole.Write(new FigletText("Server Deployment Wizard").Centered().Color(Color.Blue));
AnsiConsole.MarkupLine("[dim]v1.0[/]");
AnsiConsole.WriteLine();
AnsiConsole.Write(new Rule("[yellow]Step 1: Confirmation[/]"));
AnsiConsole.WriteLine();

var proceed = AnsiConsole.Confirm("Do you want to proceed with deployment?");
if (!proceed)
{
    AnsiConsole.MarkupLine("[red]Deployment cancelled.[/]");
    return;
}

// Step 2: Environment Selection (Hierarchical SelectionPrompt)
AnsiConsole.WriteLine();
AnsiConsole.Write(new Rule("[yellow]Step 2: Environment Selection[/]"));
AnsiConsole.WriteLine();

config.Environment = AnsiConsole.Prompt(
    new SelectionPrompt<string>()
        .Title("Select [green]target environment[/]:")
        .PageSize(8)
        .MoreChoicesText("[grey](Move up and down to reveal more environments)[/]")
        .AddChoiceGroup("Production", new[] { "us-east-1", "eu-west-1", "ap-southeast-1" })
        .AddChoiceGroup("Staging", new[] { "staging-1", "staging-2" })
        .AddChoiceGroup("Development", new[] { "local", "dev-cloud" })
);

// Step 3: Services to Deploy (MultiSelectionPrompt with groups)
AnsiConsole.WriteLine();
AnsiConsole.Write(new Rule("[yellow]Step 3: Service Selection[/]"));
AnsiConsole.WriteLine();

config.Services = AnsiConsole.Prompt(
    new MultiSelectionPrompt<string>()
        .Title("Select [green]services to deploy[/]:")
        .Required()
        .PageSize(12)
        .InstructionsText("[grey](Press [blue]<space>[/] to toggle, [green]<enter>[/] to confirm)[/]")
        .AddChoiceGroup("Backend", new[] { "API Gateway", "Auth Service", "Payment Service", "Notification Service" })
        .AddChoiceGroup("Frontend", new[] { "Web App", "Admin Dashboard" })
        .AddChoiceGroup("Infrastructure", new[] { "Redis Cache", "Message Queue" })
        .Select("API Gateway")
        .Select("Auth Service")
);

// Step 4: Server Name (TextPrompt with regex validation)
AnsiConsole.WriteLine();
AnsiConsole.Write(new Rule("[yellow]Step 4: Server Name[/]"));
AnsiConsole.WriteLine();

config.ServerName = AnsiConsole.Prompt(
    new TextPrompt<string>("Enter [green]server name[/]:")
        .PromptStyle("cyan")
        .ValidationErrorMessage("[red]Invalid name. Must be 3-20 chars, start with letter, alphanumeric + hyphens only.[/]")
        .Validate(name =>
        {
            if (string.IsNullOrWhiteSpace(name))
                return ValidationResult.Error("[red]Server name cannot be empty[/]");

            if (!Regex.IsMatch(name, @"^[a-zA-Z][a-zA-Z0-9-]{2,19}$"))
                return ValidationResult.Error("[red]Invalid format[/]");

            return ValidationResult.Success();
        })
);

// Step 5: Port Configuration (TextPrompt<int> with default and range validation)
AnsiConsole.WriteLine();
AnsiConsole.Write(new Rule("[yellow]Step 5: Port Configuration[/]"));
AnsiConsole.WriteLine();

config.Port = AnsiConsole.Prompt(
    new TextPrompt<int>("Enter [green]port number[/]:")
        .DefaultValue(8080)
        .PromptStyle("cyan")
        .ValidationErrorMessage("[red]Port must be between 1 and 65535[/]")
        .Validate(port =>
        {
            if (port < 1 || port > 65535)
                return ValidationResult.Error("[red]Port out of range[/]");

            return ValidationResult.Success();
        })
);

// Step 6: Replica Count (TextPrompt with specific choices)
AnsiConsole.WriteLine();
AnsiConsole.Write(new Rule("[yellow]Step 6: Replica Count[/]"));
AnsiConsole.WriteLine();

config.Replicas = AnsiConsole.Prompt(
    new TextPrompt<int>("Number of [green]replicas[/]:")
        .DefaultValue(3)
        .PromptStyle("cyan")
        .AddChoice(1)
        .AddChoice(2)
        .AddChoice(3)
        .AddChoice(5)
        .AddChoice(8)
        .ShowChoices()
        .ValidationErrorMessage("[red]Must be one of: 1, 2, 3, 5, 8[/]")
);

// Step 7: Database Password (Secret with asterisk mask and validation)
AnsiConsole.WriteLine();
AnsiConsole.Write(new Rule("[yellow]Step 7: Database Password[/]"));
AnsiConsole.WriteLine();

config.DbPassword = AnsiConsole.Prompt(
    new TextPrompt<string>("Enter [green]database password[/]:")
        .PromptStyle("red")
        .Secret()
        .ValidationErrorMessage("[red]Password must be at least 8 chars with a digit and special character[/]")
        .Validate(pwd =>
        {
            if (pwd.Length < 8)
                return ValidationResult.Error("[red]Too short[/]");

            if (!pwd.Any(char.IsDigit))
                return ValidationResult.Error("[red]Must contain a digit[/]");

            if (!pwd.Any(c => "!@#$%^&*()_+-=[]{}|;:,.<>?".Contains(c)))
                return ValidationResult.Error("[red]Must contain a special character[/]");

            return ValidationResult.Success();
        })
);

// Step 8: API Key (Invisible secret input)
AnsiConsole.WriteLine();
AnsiConsole.Write(new Rule("[yellow]Step 8: API Key[/]"));
AnsiConsole.WriteLine();

config.ApiKey = AnsiConsole.Prompt(
    new TextPrompt<string>("Enter [green]API key[/]:")
        .PromptStyle("red")
        .Secret(null)
        .ValidationErrorMessage("[red]API key cannot be empty[/]")
        .Validate(key =>
        {
            if (string.IsNullOrWhiteSpace(key))
                return ValidationResult.Error("[red]API key required[/]");

            return ValidationResult.Success();
        })
);

// Step 9: Log Level Selection with Search
AnsiConsole.WriteLine();
AnsiConsole.Write(new Rule("[yellow]Step 9: Log Level[/]"));
AnsiConsole.WriteLine();

config.LogLevel = AnsiConsole.Prompt(
    new SelectionPrompt<string>()
        .Title("Select [green]log level[/]:")
        .PageSize(7)
        .MoreChoicesText("[grey](Type to filter options)[/]")
        .EnableSearch()
        .AddChoices("Trace", "Debug", "Information", "Warning", "Error", "Critical", "None")
);

// Step 10: Feature Flags (Optional MultiSelectionPrompt)
AnsiConsole.WriteLine();
AnsiConsole.Write(new Rule("[yellow]Step 10: Feature Flags[/]"));
AnsiConsole.WriteLine();

config.FeatureFlags = AnsiConsole.Prompt(
    new MultiSelectionPrompt<string>()
        .Title("Enable [green]feature flags[/] (optional):")
        .NotRequired()
        .PageSize(6)
        .InstructionsText("[grey](Press [blue]<space>[/] to toggle, [green]<enter>[/] to confirm - optional)[/]")
        .AddChoices("dark-mode", "new-dashboard", "beta-api-v2", "experimental-cache", "audit-logging")
        .WrapAround()
);

// Step 11: Confirmation with Custom Yes/No
AnsiConsole.WriteLine();
AnsiConsole.Write(new Rule("[yellow]Step 11: Final Confirmation[/]"));
AnsiConsole.WriteLine();

AnsiConsole.MarkupLine($"Deploy [cyan]'{config.ServerName}'[/] to [cyan]{config.Environment}[/] with [cyan]{config.Replicas}[/] replicas?");
AnsiConsole.WriteLine();

var deploy = AnsiConsole.Prompt(
    new ConfirmationPrompt("Continue with deployment?")
    {
        Yes = 'p',
        No = 'a',
        ShowChoices = true,
        ShowDefaultValue = true,
        DefaultValue = true,
        ChoicesStyle = new Style(foreground: Color.Yellow)
    }
);

if (!deploy)
{
    AnsiConsole.MarkupLine("[red]Deployment aborted.[/]");
    return;
}

// Step 12: Deployment Progress (Multi-task Progress)
AnsiConsole.WriteLine();
AnsiConsole.Write(new Rule("[yellow]Step 12: Deployment Progress[/]"));
AnsiConsole.WriteLine();

await AnsiConsole.Progress()
    .AutoClear(false)
    .Columns(
        new TaskDescriptionColumn(),
        new ProgressBarColumn(),
        new PercentageColumn(),
        new SpinnerColumn()
    )
    .StartAsync(async ctx =>
    {
        var pullTask = ctx.AddTask("[green]Pulling images[/]");
        var migrateTask = ctx.AddTask("[yellow]Running migrations[/]");
        var startTask = ctx.AddTask("[blue]Starting services[/]");
        var healthTask = ctx.AddTask("[cyan]Health checks[/]");

        // Task 1: Pull images
        while (!pullTask.IsFinished)
        {
            await Task.Delay(50);
            pullTask.Increment(2.5);
        }

        // Task 2: Run migrations
        while (!migrateTask.IsFinished)
        {
            await Task.Delay(40);
            migrateTask.Increment(2.0);
        }

        // Task 3: Start services
        while (!startTask.IsFinished)
        {
            await Task.Delay(35);
            startTask.Increment(2.5);
        }

        // Task 4: Health checks
        while (!healthTask.IsFinished)
        {
            await Task.Delay(30);
            healthTask.Increment(3.0);
        }
    });

// Step 13: Live Status (Changing spinner and status)
AnsiConsole.WriteLine();
AnsiConsole.Write(new Rule("[yellow]Step 13: Verification[/]"));
AnsiConsole.WriteLine();

await AnsiConsole.Status()
    .Spinner(Spinner.Known.Dots)
    .SpinnerStyle(Style.Parse("green bold"))
    .StartAsync("Verifying deployment...", async ctx =>
    {
        await Task.Delay(1000);

        ctx.Status("Running health checks...");
        ctx.Spinner(Spinner.Known.Dots2);
        ctx.SpinnerStyle(Style.Parse("yellow bold"));
        await Task.Delay(1000);

        ctx.Status("Warming up caches...");
        ctx.Spinner(Spinner.Known.Dots3);
        ctx.SpinnerStyle(Style.Parse("cyan bold"));
        await Task.Delay(1000);

        ctx.Status("Finalizing...");
        ctx.Spinner(Spinner.Known.BouncingBall);
        ctx.SpinnerStyle(Style.Parse("blue bold"));
        await Task.Delay(800);
    });

// Step 14: Summary with Live Display (Live-updating table)
AnsiConsole.WriteLine();
AnsiConsole.Write(new Rule("[yellow]Step 14: Deployment Summary[/]"));
AnsiConsole.WriteLine();

var summaryData = new List<(string Service, string Status, string Endpoint)>
{
    ("API Gateway", "Running", $"http://{config.ServerName}:{config.Port}"),
    ("Auth Service", "Running", $"http://{config.ServerName}:{config.Port + 1}"),
    ("Redis Cache", "Running", $"redis://{config.ServerName}:6379"),
};

if (config.Services.Contains("Payment Service"))
    summaryData.Add(("Payment Service", "Running", $"http://{config.ServerName}:{config.Port + 2}"));

if (config.Services.Contains("Web App"))
    summaryData.Add(("Web App", "Running", $"https://{config.ServerName}.example.com"));

await AnsiConsole.Live(CreateSummaryTable(new List<(string, string, string)>()))
    .StartAsync(async ctx =>
    {
        for (int i = 0; i < summaryData.Count; i++)
        {
            await Task.Delay(300);
            ctx.UpdateTarget(CreateSummaryTable(summaryData.Take(i + 1).ToList()));
        }
    });

// Final: Results Table (Static output)
AnsiConsole.WriteLine();
AnsiConsole.Write(new Rule("[yellow]Deployment Complete[/]"));
AnsiConsole.WriteLine();

var resultsTable = new Table()
    .Border(TableBorder.Rounded)
    .BorderColor(Color.Green)
    .AddColumn(new TableColumn("[bold]Configuration[/]").Centered())
    .AddColumn(new TableColumn("[bold]Value[/]"));

resultsTable.AddRow("Environment", $"[cyan]{config.Environment}[/]");
resultsTable.AddRow("Server Name", $"[cyan]{config.ServerName}[/]");
resultsTable.AddRow("Port", $"[cyan]{config.Port}[/]");
resultsTable.AddRow("Replicas", $"[cyan]{config.Replicas}[/]");
resultsTable.AddRow("Log Level", $"[cyan]{config.LogLevel}[/]");
resultsTable.AddRow("Services", $"[cyan]{config.Services.Count}[/] deployed");
resultsTable.AddRow("Feature Flags", $"[cyan]{config.FeatureFlags.Count}[/] enabled");
resultsTable.AddRow("Status", "[green]✓ Operational[/]");

AnsiConsole.Write(resultsTable);

AnsiConsole.WriteLine();
AnsiConsole.MarkupLine($"[green]✓[/] Deployment of [cyan]{config.ServerName}[/] completed successfully!");
AnsiConsole.MarkupLine($"[dim]All {config.Services.Count} services are running on {config.Environment}[/]");

// Helper method for live table
static Table CreateSummaryTable(List<(string Service, string Status, string Endpoint)> data)
{
    var table = new Table()
        .Border(TableBorder.Rounded)
        .BorderColor(Color.Blue)
        .AddColumn(new TableColumn("[bold]Service[/]"))
        .AddColumn(new TableColumn("[bold]Status[/]"))
        .AddColumn(new TableColumn("[bold]Endpoint[/]"));

    foreach (var (service, status, endpoint) in data)
    {
        var statusMarkup = status == "Running" ? "[green]✓ Running[/]" : $"[yellow]{status}[/]";
        table.AddRow(service, statusMarkup, $"[dim]{endpoint}[/]");
    }

    return table;
}

// Configuration storage class
class DeploymentConfig
{
    public string Environment { get; set; } = "";
    public List<string> Services { get; set; } = new();
    public string ServerName { get; set; } = "";
    public int Port { get; set; }
    public int Replicas { get; set; }
    public string DbPassword { get; set; } = "";
    public string ApiKey { get; set; } = "";
    public string LogLevel { get; set; } = "";
    public List<string> FeatureFlags { get; set; } = new();
}
