{ agentSkills, ... }:
{
  programs.antigravity-cli = {
    enable = true;
    skills = agentSkills;
  };
}
