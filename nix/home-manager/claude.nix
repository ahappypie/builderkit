{ agentSkills, ... }:
{
  programs.claude-code = {
    enable = true;
    skills = agentSkills;
  };
}
